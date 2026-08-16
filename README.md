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

Select an output path and exclude repository-relative glob patterns:

```bash
repomd -w ./my-project -o context.md -e 'target/**,node_modules/**'
```

The command stops if it cannot read a path. Use `--best-effort` to continue and print a skipped-path report. Use `--force` to replace an existing output file.

Set optional byte limits for large repositories:

```bash
repomd -w ./my-project --max-file-size 1000000 --max-total-size 10000000
```

After success, the command reports included files, source and output bytes, excluded files, binary files, files over the size limit, and read errors.

Estimate tokens and stop on common secret formats:

```bash
repomd -w ./my-project --estimate-tokens --check-secrets
```

The token value is a rough estimate of one token per four source bytes. The secret check detects common private-key, AWS, GitHub, and `sk-` key formats. It cannot detect every secret.

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
repomd -w ./my-project -o context.md -e 'target/**,node_modules/**'
repomd -w ./my-project --max-file-size 1000000 --max-total-size 10000000
repomd -w ./my-project --estimate-tokens --check-secrets
```

Флаг `--exclude` принимает glob-шаблоны относительно корня репозитория. По умолчанию команда остановится, если файл нельзя прочитать. Флаг `--best-effort` разрешает продолжить работу. Флаг `--force` разрешает заменить существующий результат. `--estimate-tokens` показывает приблизительное число токенов. `--check-secrets` ищет распространенные форматы ключей, но не может найти каждый секрет. После успешной работы команда показывает сводку.
