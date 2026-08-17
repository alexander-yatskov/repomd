use anyhow::{Context, bail};
use clap::Parser;
use content_inspector::{ContentType, inspect};
use ignore::{WalkBuilder, overrides::OverrideBuilder};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf, absolute};

#[derive(Parser, Debug)]
#[command(
    name = "repomd",
    version,
    about = "Create a Markdown snapshot of a repository for AI tools"
)]
struct Args {
    #[arg(short, long, default_value = ".")]
    workdirectory: PathBuf,
    /// Repository-relative glob patterns to exclude
    #[arg(short, long, value_delimiter = ',')]
    exclude: Vec<String>,
    #[arg(short, long)]
    ignore_gitignore: bool,
    #[arg(short, long)]
    output: Option<PathBuf>,
    #[arg(short = 'd', long)]
    max_depth: Option<usize>,
    #[arg(short, long, default_value = "Source")]
    prefix: String,
    /// Continue when a directory entry or file cannot be read
    #[arg(long)]
    best_effort: bool,
    /// Replace an existing output file
    #[arg(long)]
    force: bool,
    /// Skip files larger than this number of bytes
    #[arg(long)]
    max_file_size: Option<u64>,
    /// Limit included source content to this number of bytes
    #[arg(long)]
    max_total_size: Option<u64>,
    /// Print a rough token estimate based on four source bytes per token
    #[arg(long)]
    estimate_tokens: bool,
    /// Stop if an included file contains a common secret format
    #[arg(long)]
    check_secrets: bool,
}

fn main() -> anyhow::Result<()> {
    let report = pack(&Args::parse())?;
    println!("Created: {}", report.output.display());
    println!("Files: {}", report.files);
    println!("Source bytes: {}", report.source_bytes);
    println!("Output bytes: {}", report.output_bytes);
    println!("Excluded: {}", report.excluded);
    println!("Binary: {}", report.binary);
    println!("Over size limit: {}", report.over_size_limit);
    if let Some(tokens) = report.estimated_tokens {
        println!("Estimated tokens: {tokens}");
    }
    if !report.skipped.is_empty() {
        eprintln!("Skipped {} path(s):", report.skipped.len());
        for path in report.skipped {
            eprintln!("  {path}");
        }
    }
    Ok(())
}

#[derive(Default)]
struct Report {
    output: PathBuf,
    skipped: Vec<String>,
    files: u64,
    source_bytes: u64,
    output_bytes: u64,
    excluded: u64,
    binary: u64,
    over_size_limit: u64,
    estimated_tokens: Option<u64>,
}

fn pack(args: &Args) -> anyhow::Result<Report> {
    let root = absolute(&args.workdirectory)
        .with_context(|| format!("cannot resolve {}", args.workdirectory.display()))?;
    if !root.is_dir() {
        bail!("work directory is not a directory: {}", root.display());
    }

    let folder_name = root
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "project".into());
    let mut output = args.output.clone().unwrap_or_else(|| PathBuf::from("."));
    if output.is_dir() {
        output.push(format!("{}_{}.md", args.prefix, folder_name));
    }
    let output =
        absolute(&output).with_context(|| format!("cannot resolve {}", output.display()))?;
    if output.exists() && !args.force {
        bail!(
            "output already exists: {} (use --force to replace it)",
            output.display()
        );
    }

    let (temp_path, mut out_file) = create_temp_file(&output)?;
    let mut report = match write_snapshot(args, &root, &output, &temp_path, &mut out_file) {
        Ok(report) => report,
        Err(error) => {
            drop(out_file);
            let _ = fs::remove_file(&temp_path);
            return Err(error);
        }
    };
    out_file.flush()?;
    out_file.sync_all()?;
    drop(out_file);

    if output.exists() {
        fs::remove_file(&output).with_context(|| format!("cannot replace {}", output.display()))?;
    }
    fs::rename(&temp_path, &output)
        .with_context(|| format!("cannot move output to {}", output.display()))?;
    report.output_bytes = fs::metadata(&output)?.len();
    report.output = output;
    Ok(report)
}

fn create_temp_file(output: &Path) -> anyhow::Result<(PathBuf, File)> {
    let parent = output.parent().unwrap_or_else(|| Path::new("."));
    let name = output.file_name().unwrap_or_default().to_string_lossy();
    for number in 0..100 {
        let path = parent.join(format!(".{name}.{}.{}.tmp", std::process::id(), number));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error).context("cannot create temporary output file"),
        }
    }
    bail!("cannot create a unique temporary output file")
}

fn write_snapshot(
    args: &Args,
    root: &Path,
    output: &Path,
    temp_path: &Path,
    out_file: &mut File,
) -> anyhow::Result<Report> {
    let mut walker = WalkBuilder::new(root);
    walker
        .standard_filters(!args.ignore_gitignore)
        .follow_links(false)
        .max_depth(args.max_depth)
        .sort_by_file_path(|a, b| a.cmp(b));

    let mut excludes = OverrideBuilder::new(root);
    for pattern in &args.exclude {
        excludes
            .add(&format!("!{pattern}"))
            .with_context(|| format!("invalid exclude glob: {pattern}"))?;
    }
    let excludes = excludes.build()?;
    let mut report = Report::default();
    for result in walker.build() {
        let entry = match result {
            Ok(entry) => entry,
            Err(error) if args.best_effort => {
                report.skipped.push(error.to_string());
                continue;
            }
            Err(error) => return Err(error).context("cannot scan work directory"),
        };
        let path = entry.path();
        if path.is_dir() || absolute(path).is_ok_and(|path| path == output || path == temp_path) {
            continue;
        }
        let relative = path.strip_prefix(root).unwrap_or(path);
        if excludes.matched(path, false).is_ignore() {
            report.excluded += 1;
            continue;
        }

        if args.max_file_size.is_some_and(|limit| {
            entry
                .metadata()
                .is_ok_and(|metadata| metadata.len() > limit)
        }) {
            report.over_size_limit += 1;
            continue;
        }

        let buffer = match fs::read(path) {
            Ok(buffer) => buffer,
            Err(error) if args.best_effort => {
                report
                    .skipped
                    .push(format!("{}: {error}", relative.display()));
                continue;
            }
            Err(error) => {
                return Err(error).with_context(|| format!("cannot read {}", path.display()));
            }
        };
        let size = buffer.len() as u64;
        if args.max_file_size.is_some_and(|limit| size > limit)
            || args
                .max_total_size
                .is_some_and(|limit| report.source_bytes.saturating_add(size) > limit)
        {
            report.over_size_limit += 1;
            continue;
        }
        if inspect(&buffer[..buffer.len().min(1024)]) == ContentType::BINARY {
            report.binary += 1;
            continue;
        }
        let content = String::from_utf8_lossy(&buffer);
        if args.check_secrets && contains_possible_secret(&content) {
            bail!(
                "possible secret found in {} (review the file or run without --check-secrets)",
                relative.display()
            );
        }
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let fence = markdown_fence(&content);
        writeln!(out_file, "## File: {}", relative.display())?;
        writeln!(out_file, "{fence}{extension}")?;
        writeln!(out_file, "{content}")?;
        writeln!(out_file, "{fence}\n")?;
        writeln!(out_file, "---\n")?;
        report.files += 1;
        report.source_bytes += size;
    }
    if args.estimate_tokens {
        report.estimated_tokens = Some(report.source_bytes.div_ceil(4));
    }
    Ok(report)
}

fn contains_possible_secret(content: &str) -> bool {
    const PRIVATE_KEYS: [&str; 4] = [
        "-----BEGIN PRIVATE KEY-----",
        "-----BEGIN RSA PRIVATE KEY-----",
        "-----BEGIN EC PRIVATE KEY-----",
        "-----BEGIN OPENSSH PRIVATE KEY-----",
    ];
    if PRIVATE_KEYS.iter().any(|marker| content.contains(marker)) {
        return true;
    }

    content
        .split(|character: char| {
            character.is_whitespace() || matches!(character, '\'' | '\"' | '`' | '=' | ':' | ',')
        })
        .any(|token| {
            (token.starts_with("AKIA") && token.len() >= 20)
                || (token.starts_with("ASIA") && token.len() >= 20)
                || (token.starts_with("ghp_") && token.len() >= 20)
                || (token.starts_with("github_pat_") && token.len() >= 30)
                || (token.starts_with("sk-") && token.len() >= 20)
        })
}

fn markdown_fence(content: &str) -> String {
    let longest = content
        .split(|character| character != '`')
        .map(str::len)
        .max()
        .unwrap_or(0);
    "`".repeat(3.max(longest + 1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    fn test_directory() -> PathBuf {
        loop {
            let path = std::env::temp_dir().join(format!(
                "repomd-{}-{}",
                std::process::id(),
                NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => return path,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("cannot create test directory: {error}"),
            }
        }
    }

    fn args(root: &Path, output: &Path) -> Args {
        Args {
            workdirectory: root.into(),
            exclude: Vec::new(),
            ignore_gitignore: false,
            output: Some(output.into()),
            max_depth: None,
            prefix: "Source".into(),
            best_effort: false,
            force: false,
            max_file_size: None,
            max_total_size: None,
            estimate_tokens: false,
            check_secrets: false,
        }
    }

    #[test]
    fn uses_a_longer_fence_for_embedded_backticks() {
        assert_eq!(markdown_fence("text\n```rust\ncode\n```"), "````");
        assert_eq!(markdown_fence("text"), "```");
    }

    #[test]
    fn creates_stable_markdown_and_does_not_include_output() {
        let root = test_directory();
        fs::write(root.join("b.rs"), "fn b() {}\n").unwrap();
        fs::write(root.join("a.md"), "example:\n```\ntext\n```\n").unwrap();
        let output = root.join("snapshot.md");
        pack(&args(&root, &output)).unwrap();

        let first = fs::read_to_string(&output).unwrap();
        assert!(first.find("a.md").unwrap() < first.find("b.rs").unwrap());
        assert!(first.contains("````md"));
        assert!(!first.contains("## File: snapshot.md"));
        assert!(!first.contains(".tmp"));

        let mut second_run = args(&root, &output);
        second_run.force = true;
        pack(&second_run).unwrap();
        assert_eq!(first, fs::read_to_string(&output).unwrap());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn keeps_existing_output_without_force() {
        let root = test_directory();
        fs::write(root.join("input.txt"), "input").unwrap();
        let output = root.join("snapshot.md");
        fs::write(&output, "keep me").unwrap();
        assert!(pack(&args(&root, &output)).is_err());
        assert_eq!(fs::read_to_string(&output).unwrap(), "keep me");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn applies_relative_globs_and_size_limits() {
        let root = test_directory();
        fs::create_dir(root.join("target")).unwrap();
        fs::write(root.join("target/generated.txt"), "generated").unwrap();
        fs::write(root.join("mytarget.txt"), "keep").unwrap();
        fs::write(root.join("large.txt"), "too large").unwrap();
        let output = root.join("snapshot.md");
        let mut options = args(&root, &output);
        options.exclude.push("target/**".into());
        options.max_file_size = Some(8);

        let report = pack(&options).unwrap();
        let content = fs::read_to_string(&output).unwrap();
        assert!(content.contains("mytarget.txt"));
        assert!(!content.contains("generated.txt"));
        assert!(!content.contains("large.txt"));
        assert_eq!(report.excluded, 1);
        assert_eq!(report.over_size_limit, 1);

        fs::remove_file(&output).unwrap();
        let output = root.join("total.md");
        let mut options = args(&root, &output);
        options.exclude.push("target/**".into());
        options.max_total_size = Some(5);
        let report = pack(&options).unwrap();
        assert_eq!(report.files, 1);
        assert_eq!(report.source_bytes, 4);
        assert_eq!(report.over_size_limit, 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn estimates_tokens_and_blocks_common_secrets() {
        assert!(contains_possible_secret("key = AKIA1234567890123456"));
        assert!(contains_possible_secret(
            "-----BEGIN PRIVATE KEY-----\ndata"
        ));
        assert!(!contains_possible_secret("let key = \"example\";"));

        let root = test_directory();
        fs::write(root.join("input.txt"), "12345678").unwrap();
        let output = root.join("snapshot.md");
        let mut options = args(&root, &output);
        options.estimate_tokens = true;
        let report = pack(&options).unwrap();
        assert_eq!(report.estimated_tokens, Some(2));
        fs::remove_dir_all(root).unwrap();
    }
}
