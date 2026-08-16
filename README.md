# repomd

[![Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

`repomd` creates a local, `.gitignore`-aware Markdown snapshot of a codebase for review before manual upload to NotebookLM, ChatGPT, or Claude.

It skips binary files, does not follow symbolic links, and uses safe Markdown fences. It does not upload data.

> Review the output before upload. A file that is not ignored can contain a secret.

## Install

Download a binary from the [release page](https://github.com/alexander-yatskov/repomd/releases), or build it:

```bash
git clone https://github.com/alexander-yatskov/repomd.git
cd repomd
cargo build --release
```

## Use

Create `Source_my-project.md` in the current directory:

```bash
repomd --workdirectory ./my-project
```

Select an output path and exclude matching paths:

```bash
repomd -w ./my-project -o context.md -e target,node_modules
```

The command stops if it cannot read a path. Use `--best-effort` to continue and print a skipped-path report. Use `--force` to replace an existing output file.

Run `repomd --help` for all options.

## RU

`repomd` создает локальный Markdown-снимок кодовой базы. Он учитывает `.gitignore`. Вы можете проверить файл перед ручной загрузкой в NotebookLM, ChatGPT или Claude.

Программа пропускает бинарные файлы, не переходит по символическим ссылкам и не загружает данные в сеть.

> Проверьте результат перед загрузкой. Файл, который не исключен, может содержать секрет.

### Установка

Загрузите бинарный файл со [страницы релизов](https://github.com/alexander-yatskov/repomd/releases) или соберите проект:

```bash
git clone https://github.com/alexander-yatskov/repomd.git
cd repomd
cargo build --release
```

### Использование

```bash
repomd --workdirectory ./my-project
repomd -w ./my-project -o context.md -e target,node_modules
```

По умолчанию команда остановится, если файл нельзя прочитать. Флаг `--best-effort` разрешает продолжить работу. Флаг `--force` разрешает заменить существующий результат.
