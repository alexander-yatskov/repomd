# repomd

**Local code context you can inspect before it leaves your machine.**

`repomd` turns the relevant text files in a codebase into one deterministic Markdown snapshot. Review it, then upload it manually to NotebookLM, ChatGPT, Claude, or another AI tool.

- Local: no account, server, or upload.
- Reviewable: plain Markdown with a clear section for each file.
- Repository-aware: respects `.gitignore` by default.
- Predictable: stable file order, glob exclusions, and byte limits.
- Safer: skips binary files and can check common secret formats.

## Install

From crates.io:

```bash
cargo install repomd-cli --locked
```

The package is named `repomd-cli`. The installed command is `repomd`.

You can also download a binary from [GitHub Releases](https://github.com/alexander-yatskov/repomd/releases) or build from source:

```bash
git clone https://github.com/alexander-yatskov/repomd.git
cd repomd
cargo build --release --locked
```

## Quick start

Create `Source_my-project.md` in the current directory:

```bash
repomd --workdirectory ./my-project
```

Create a selected snapshot:

```bash
repomd \
  --workdirectory ./my-project \
  --output context.md \
  --exclude 'target/**,node_modules/**' \
  --max-file-size 1000000 \
  --max-total-size 10000000 \
  --estimate-tokens \
  --check-secrets
```

Run `repomd --help` for the complete CLI reference.

## Safety model

The command stops on read errors by default. Use `--best-effort` only when a partial snapshot is acceptable.

An existing output file is not replaced unless you use `--force`. Output is first written to a temporary file. The output file and temporary file are never included in the snapshot.

`--check-secrets` detects common private-key, AWS, GitHub, and `sk-` key formats. It is a heuristic check. It cannot prove that a snapshot has no secrets. Always review the result before upload.

## Limits and summary

`--max-file-size` skips a file above the specified byte limit. `--max-total-size` limits the combined source bytes included in the snapshot.

After success, `repomd` reports:

- included files and source bytes;
- output bytes;
- excluded and binary files;
- files skipped by size limits;
- read errors in best-effort mode;
- an optional token estimate.

The token estimate uses one token per four source bytes. It is useful for a quick size check, not for billing or exact model limits.

## RU

**Локальный контекст кода, который можно проверить до передачи во внешний AI-инструмент.**

`repomd` собирает нужные текстовые файлы кодовой базы в один стабильный Markdown-файл. Программа работает локально, учитывает `.gitignore`, пропускает бинарные файлы и ничего не загружает в сеть.

### Установка

```bash
cargo install repomd-cli --locked
```

Пакет на crates.io называется `repomd-cli`. Установленная команда называется `repomd`.

### Быстрый старт

```bash
repomd --workdirectory ./my-project

repomd \
  --workdirectory ./my-project \
  --output context.md \
  --exclude 'target/**,node_modules/**' \
  --max-file-size 1000000 \
  --max-total-size 10000000 \
  --estimate-tokens \
  --check-secrets
```

По умолчанию команда останавливается при ошибке чтения. `--best-effort` разрешает неполный результат. `--force` разрешает заменить существующий файл.

Проверка `--check-secrets` находит только распространенные форматы ключей. Она не гарантирует отсутствие секретов. Всегда проверяйте итоговый Markdown-файл перед загрузкой.

## License

[MIT](https://github.com/alexander-yatskov/repomd/blob/main/LICENSE)
