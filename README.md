# PinkDown

PinkDown is a native, split-pane Markdown reader and editor written in Rust. It uses the warm, restrained [Rosé Pine](https://rosepinetheme.com/) palette and works on Windows x64 and macOS.

## Run locally

```bash
cargo run --release
```

## Build

```bash
# Windows x64
cargo build --release --target x86_64-pc-windows-msvc

# Apple Silicon macOS (run on macOS with the target installed)
rustup target add aarch64-apple-darwin
cargo build --release --target aarch64-apple-darwin

# Intel macOS
rustup target add x86_64-apple-darwin
cargo build --release --target x86_64-apple-darwin
```

The compiled binary is placed in `target/<target>/release/`.

## Features

- Side-by-side source and rendered preview
- Open, save and save-as Markdown files
- Changes indicator and keyboard shortcuts (`Ctrl/Cmd+O`, `Ctrl/Cmd+S`)
- Markdown headings, emphasis, inline code, links, lists, task items, block quotes, dividers and fenced code blocks
