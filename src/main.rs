use anyhow::{Context, bail};
use clap::Parser;
use content_inspector::{ContentType, inspect};
use ignore::WalkBuilder;
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
}

fn main() -> anyhow::Result<()> {
    let report = pack(&Args::parse())?;
    println!("Created: {}", report.output.display());
    if !report.skipped.is_empty() {
        eprintln!("Skipped {} path(s):", report.skipped.len());
        for path in report.skipped {
            eprintln!("  {path}");
        }
    }
    Ok(())
}

struct Report {
    output: PathBuf,
    skipped: Vec<String>,
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
    let skipped = match write_snapshot(args, &root, &output, &temp_path, &mut out_file) {
        Ok(skipped) => skipped,
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

    Ok(Report { output, skipped })
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
) -> anyhow::Result<Vec<String>> {
    let mut walker = WalkBuilder::new(root);
    walker
        .standard_filters(!args.ignore_gitignore)
        .follow_links(false)
        .max_depth(args.max_depth)
        .sort_by_file_path(|a, b| a.cmp(b));

    let mut skipped = Vec::new();
    for result in walker.build() {
        let entry = match result {
            Ok(entry) => entry,
            Err(error) if args.best_effort => {
                skipped.push(error.to_string());
                continue;
            }
            Err(error) => return Err(error).context("cannot scan work directory"),
        };
        let path = entry.path();
        if path.is_dir() || absolute(path).is_ok_and(|path| path == output || path == temp_path) {
            continue;
        }
        let relative = path.strip_prefix(root).unwrap_or(path);
        if args
            .exclude
            .iter()
            .any(|excluded| relative.to_string_lossy().contains(excluded))
        {
            continue;
        }

        let buffer = match fs::read(path) {
            Ok(buffer) => buffer,
            Err(error) if args.best_effort => {
                skipped.push(format!("{}: {error}", relative.display()));
                continue;
            }
            Err(error) => {
                return Err(error).with_context(|| format!("cannot read {}", path.display()));
            }
        };
        if inspect(&buffer[..buffer.len().min(1024)]) == ContentType::BINARY {
            continue;
        }
        let content = String::from_utf8_lossy(&buffer);
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
    }
    Ok(skipped)
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
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_directory() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "repomd-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        path
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
}
