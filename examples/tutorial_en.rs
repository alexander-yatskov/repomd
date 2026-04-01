// The 'clap' library turns a struct into a full-featured CLI.
// It automatically generates --help and parses command-line arguments.
use clap::Parser;

// 'content_inspector' helps determine if a file is text-based or binary (image, exe).
// This prevents us from writing "garbage" binary data into a text Markdown file.
use content_inspector::{ContentType, inspect};

// 'ignore' is a "smart" directory walker.
// It knows how to read .gitignore files and skip hidden files automatically.
use ignore::WalkBuilder;

// Rust Standard Library modules for I/O and file system operations.
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

/// This struct defines the flags and arguments our program accepts.
/// #[derive(Parser)] is a macro that generates the parsing logic for us.
#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "A utility to bundle project context into a single file"
)]
struct Args {
    /// Positional argument: the directory path.
    /// PathBuf is a cross-platform type for paths (works on Windows/Linux/macOS).
    #[arg(default_value = ".")]
    workdirectory: PathBuf,

    /// List of exclusions. 'value_delimiter' allows: --exclude "node_modules,target".
    #[arg(short, long, value_delimiter = ',')]
    exclude: Vec<String>,

    /// A flag (boolean). If present in the command, the value becomes true.
    #[arg(long, default_value_t = false)]
    ignore_gitignore: bool,

    /// Optional path for the resulting file. 'Option' means it might be None.
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Maximum recursion depth. 'usize' is an unsigned integer type.
    #[arg(short = 'd', long)]
    max_depth: Option<usize>,
}

fn main() -> anyhow::Result<()> {
    // 1. Initialize arguments. If the user input is invalid,
    // the program will print an error and exit here.
    let args = Args::parse();

    // 2. Determine the output filename.
    // If the user didn't specify a path with -o, we use the current folder name.
    let folder_name = args
        .workdirectory
        .canonicalize() // Convert to absolute path (removes "." or "..")
        .ok() // Proceed if the directory exists
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "project".into()); // Fallback if name is unavailable

    let output_path = args
        .output
        .unwrap_or_else(|| PathBuf::from(format!("Source_{}.md", folder_name)));

    // 3. Create the file where the result will be written.
    // 'File::create' overwrites the file if it already exists.
    let mut out_file = File::create(&output_path)?;

    // 4. Configure the directory scanner (Walker).
    // We use a Builder pattern to apply settings from our CLI arguments.
    let mut walker = WalkBuilder::new(&args.workdirectory);
    walker
        .standard_filters(!args.ignore_gitignore) // Toggle .gitignore support
        .follow_links(false) // Security: don't follow symlinks
        .max_depth(args.max_depth); // Limit depth if specified

    println!("Scanning directory: {:?}", args.workdirectory);

    // 5. Main traversal loop. 'build()' creates an iterator.
    for result in walker.build() {
        // 'result' might be an error (e.g., no permission), so we use 'match'.
        let entry = match result {
            Ok(e) => e,
            Err(err) => {
                eprintln!("Access error: {}", err);
                continue;
            }
        };

        let path = entry.path();

        // Skip directories (we want files) and the output file itself
        // to avoid writing the result into the source data.
        if path.is_dir() || path == output_path {
            continue;
        }

        // Check if the path contains any strings from the exclusion list.
        if args
            .exclude
            .iter()
            .any(|ex| path.to_string_lossy().contains(ex))
        {
            continue;
        }

        // 6. Handle the file content.
        if let Ok(mut file) = File::open(path) {
            // Read the first 1024 bytes. This is enough to detect the file type.
            let mut chunk = [0u8; 1024];
            let n = file.read(&mut chunk)?;

            // If content_inspector confirms it's NOT binary, it's text or code.
            if inspect(&chunk[..n]) != ContentType::BINARY {
                let mut buffer = Vec::new();
                buffer.extend_from_slice(&chunk[..n]); // Add the first chunk
                file.read_to_end(&mut buffer)?; // Read the remaining content

                // Convert bytes to a string. 'lossy' replaces invalid UTF-8
                // characters with a placeholder instead of crashing.
                let content = String::from_utf8_lossy(&buffer);

                // Get the relative path for a clean header in the Markdown.
                let rel_path = path.strip_prefix(&args.workdirectory).unwrap_or(path);

                // Get the file extension (e.g., "rs") for Markdown syntax highlighting.
                let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");

                // 7. Write data to the output file.
                // 'writeln!' macro works like 'println!' but writes to a file stream.
                writeln!(out_file, "## File: {}", rel_path.display())?;
                writeln!(out_file, "```{}", ext)?;
                writeln!(out_file, "{}", content)?;
                writeln!(out_file, "```\n")?;
                writeln!(out_file, "---\n")?;
            }
        }
    }

    println!("Success! Context saved to {}", output_path.display());
    Ok(()) // Return empty Result indicating success.
}
