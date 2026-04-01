// Библиотека clap превращает структуру в полноценный CLI-интерфейс.
// Она сама генерирует --help и парсит аргументы из командной строки.
use clap::Parser;

// content_inspector помогает понять, текстовый файл перед нами или бинарный (картинка, exe).
// Это важно, чтобы не пытаться записать "мусор" в текстовый Markdown.
use content_inspector::{ContentType, inspect};

// ignore — это "умный" аналог стандартного обхода папок.
// Он знает, как читать .gitignore и пропускать скрытые файлы.
use ignore::WalkBuilder;

// Стандартная библиотека Rust для работы с вводом-выводом и файлами.
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

/// Эта структура определяет, какие флаги и аргументы принимает наша программа.
/// #[derive(Parser)] — магическая аннотация, которая реализует логику парсинга за нас.
#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Утилита для сборки контекста проекта в один файл"
)]
struct Args {
    /// Первый позиционный аргумент: путь к папке.
    /// PathBuf — это кроссплатформенный тип для путей (работает и в Windows, и в Linux).
    #[arg(default_value = ".")]
    workdirectory: PathBuf,

    /// Список исключений. value_delimiter позволяет писать --exclude "node_modules,target".
    #[arg(short, long, value_delimiter = ',')]
    exclude: Vec<String>,

    /// Флаг (булево значение). Если он есть в команде, значение будет true.
    #[arg(long, default_value_t = false)]
    ignore_gitignore: bool,

    /// Опциональный путь к итоговому файлу. Option означает, что значения может не быть.
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Максимальная глубина рекурсии. usize — это беззнаковое целое число.
    #[arg(short = 'd', long)]
    max_depth: Option<usize>,
}

fn main() -> anyhow::Result<()> {
    // 1. Инициализируем аргументы. Если пользователь ввел что-то не то, программа
    // выведет ошибку и завершится прямо здесь.
    let args = Args::parse();

    // 2. Определяем имя выходного файла.
    // Если пользователь не указал путь через -o, мы берем имя текущей папки.
    let folder_name = args
        .workdirectory
        .canonicalize() // Превращаем путь в абсолютный (убираем "." или "..")
        .ok() // Если папка существует, продолжаем
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "project".into()); // Запасной вариант, если имя не определилось

    let output_path = args
        .output
        .unwrap_or_else(|| PathBuf::from(format!("Source_{}.md", folder_name)));

    // 3. Создаем файл, куда будем записывать результат.
    // File::create заменяет файл, если он уже существовал.
    let mut out_file = File::create(&output_path)?;

    // 4. Настраиваем "сканер" (Walker).
    // Мы создаем строитель (Builder) и передаем ему наши настройки из аргументов CLI.
    let mut walker = WalkBuilder::new(&args.workdirectory);
    walker
        .standard_filters(!args.ignore_gitignore) // Включаем/выключаем поддержку .gitignore
        .follow_links(false) // Защита от бесконечных циклов в симлинках
        .max_depth(args.max_depth); // Ограничение глубины, если задано

    println!("Начинаю сканирование: {:?}", args.workdirectory);

    // 5. Основной цикл обхода. build() превращает настройки в итератор.
    for result in walker.build() {
        // result может быть ошибкой (например, нет прав на чтение папки), поэтому используем match.
        let entry = match result {
            Ok(e) => e,
            Err(err) => {
                eprintln!("Ошибка доступа: {}", err);
                continue;
            }
        };

        let path = entry.path();

        // Пропускаем директории (нам нужны только файлы) и сам файл результата,
        // чтобы не записывать его в самого себя.
        if path.is_dir() || path == output_path {
            continue;
        }

        // Простая проверка: содержит ли путь строки из списка исключений (--exclude).
        if args
            .exclude
            .iter()
            .any(|ex| path.to_string_lossy().contains(ex))
        {
            continue;
        }

        // 6. Работа с файлом.
        if let Ok(mut file) = File::open(path) {
            // Читаем первые 1024 байта. Этого достаточно, чтобы понять тип файла.
            let mut chunk = [0u8; 1024];
            let n = file.read(&mut chunk)?;

            // Если content_inspector говорит, что это НЕ бинарник — значит, это текст или код.
            if inspect(&chunk[..n]) != ContentType::BINARY {
                let mut buffer = Vec::new();
                buffer.extend_from_slice(&chunk[..n]); // Добавляем уже прочитанный кусочек
                file.read_to_end(&mut buffer)?; // Дочитываем остальное до конца

                // Превращаем байты в строку. lossy означает, что если встретится
                // битый символ, программа не "упадет", а заменит его на спецсимвол.
                let content = String::from_utf8_lossy(&buffer);

                // Получаем путь файла относительно корня сканирования для красивого заголовка.
                let rel_path = path.strip_prefix(&args.workdirectory).unwrap_or(path);

                // Берем расширение (например, "rs" или "py") для подсветки синтаксиса в Markdown.
                let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");

                // 7. Записываем данные в итоговый файл.
                // Используем макрос writeln!, который работает как println!, но пишет в файл.
                writeln!(out_file, "## File: {}", rel_path.display())?;
                writeln!(out_file, "```{}", ext)?;
                writeln!(out_file, "{}", content)?;
                writeln!(out_file, "```\n")?;
                writeln!(out_file, "---\n")?;
            }
        }
    }

    println!("Успешно! Результат сохранен в {}", output_path.display());
    Ok(()) // Возвращаем пустой результат, означающий, что всё прошло успешно.
}
