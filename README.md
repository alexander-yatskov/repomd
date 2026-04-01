# repomd

[![Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**repomd** is a high-performance CLI utility that recursively scans your directory and consolidates all text and source code files into a single, structured Markdown file.

Specifically designed to create a perfect **Source File** for Google NotebookLM, ChatGPT, or Claude when you need to provide full project context in one go.

## Features

- **Smart Content Detection:** Uses `content_inspector` to automatically skip binary files, images, and audio.
- **Git-Aware:** Respects your `.gitignore` rules by default (skips `node_modules`, `target`, logs, etc.).
- **Safe Traversal:** Does not follow symbolic links (symlinks) to prevent infinite recursion and loops.
- **NotebookLM Optimized:** Formats output with clear H2 headers and fenced code blocks for superior LLM parsing.

## Installation

Build from source using Cargo:

```bash
git clone [https://github.com/yourusername/repomd.git](https://github.com/yourusername/repomd.git)
cd repomd
cargo build --release
```

Folder ```examples``` contains fully commented source code for learning purposes

# RU

[![Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**repomd** — это быстрая CLI-утилита, которая рекурсивно сканирует вашу директорию и объединяет все текстовые файлы и исходный код в один структурированный Markdown-файл. 

Идеально подходит для создания **Source File** для Google NotebookLM, ChatGPT или Claude, когда нужно передать контекст всего проекта целиком.

## Особенности

- **Умное определение типов:** Использует `content_inspector`, чтобы автоматически игнорировать бинарные файлы, изображения и аудио.
- **Уважение к `.gitignore`:** По умолчанию не включает файлы, которые вы скрыли от git (node_modules, target, логи).
- **Безопасность:** Не переходит по символическим ссылкам (symlinks), предотвращая бесконечные циклы.
- **NotebookLM Ready:** Форматирует вывод с четкими заголовками и блоками кода для лучшего парсинга нейросетями.

## Установка

Пока проект находится в разработке, вы можете собрать его из исходников:

```bash
git clone [https://github.com/yourusername/repomd.git](https://github.com/yourusername/repomd.git)
cd repomd
cargo build --release
```

В директории ```examples``` можно найти полностью комментированный код для обучения 
