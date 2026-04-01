use clap::Parser;
use content_inspector::{ContentType, inspect};
use ignore::WalkBuilder;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "repomd",
    about = "Конвертирует репозиторий в один Markdown файл"
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
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Определение имени выходного файла
    let folder_name = args
        .workdirectory
        .canonicalize()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "project".into());

    let mut output_path = args.output.unwrap_or_else(|| PathBuf::from("."));

    if output_path.is_dir() {
        let file_name = format!("{}_{}.md", args.prefix, folder_name);
        output_path.push(file_name);
    }

    let mut out_file = File::create(&output_path)?;

    // Настройка обхода
    let mut walker = WalkBuilder::new(&args.workdirectory);
    walker
        .standard_filters(!args.ignore_gitignore) // Включает .gitignore и скрытые файлы
        .follow_links(false) // Не ходим по симлинкам
        .max_depth(args.max_depth);

    for result in walker.build() {
        let entry = match result {
            Ok(e) => e,
            Err(err) => {
                eprintln!("Ошибка доступа: {}", err);
                continue;
            }
        };

        let path = entry.path();

        // Пропускаем директории и сам файл результата
        if path.is_dir() || path == output_path {
            continue;
        }

        // Ручной фильтр исключений
        if args
            .exclude
            .iter()
            .any(|ex| path.to_string_lossy().contains(ex))
        {
            continue;
        }

        if let Ok(mut file) = File::open(path) {
            let mut buffer = Vec::new();
            // Читаем начало для проверки типа
            let mut chunk = [0u8; 1024];
            let n = file.read(&mut chunk)?;

            if inspect(&chunk[..n]) != ContentType::BINARY {
                // Читаем весь файл
                buffer.extend_from_slice(&chunk[..n]);
                file.read_to_end(&mut buffer)?;

                let content = String::from_utf8_lossy(&buffer);
                let rel_path = path.strip_prefix(&args.workdirectory).unwrap_or(path);
                let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");

                writeln!(out_file, "## File: {}", rel_path.display())?;
                writeln!(out_file, "```{}", ext)?;
                writeln!(out_file, "{}", content)?;
                writeln!(out_file, "```\n")?;
                writeln!(out_file, "---\n")?; // Разделитель для визуальной чистоты
            }
        }
    }

    println!("Готово! Файл сформирован: {}", output_path.display());
    Ok(())
}
