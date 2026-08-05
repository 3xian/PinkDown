<p align="center">
  <img src="assets/pinkdown-macos-icon.png" width="144" height="144" alt="PinkDown app icon">
</p>

# PinkDown

PinkDown is a fast, sleek, native split-pane Markdown editor and reader for Windows and macOS, built in Rust for instant startup and a polished desktop experience. Edit Markdown source beside a live preview in a calm Rosé Pine interface—without a browser, account, or workspace setup.

[Website](https://3xian.github.io/PinkDown/) · [Download the latest release](https://github.com/3xian/PinkDown/releases/latest)

![PinkDown editor showing Markdown source and live preview](docs/assets/pinkdown-screenshot.webp)

## Highlights

- Side-by-side Markdown source and rendered preview
- Open, save, and save-as support for `.md`, `.markdown`, `.mdx`, and `.txt` files
- Export to self-contained HTML, or to PDF via a system Chromium browser (Edge / Chrome)
- Clear unsaved-change indicator and familiar `Ctrl/Cmd + O` / `Ctrl/Cmd + S` / `Ctrl/Cmd + Shift + S` shortcuts
- Choose a system typeface for the UI and preview (persisted across launches)
- Markdown rendering for headings, emphasis, links, inline and fenced code, lists, task items, block quotes, dividers, and tables
- UTF-8 and UTF-16 file decoding with encoding feedback in the status bar
- Native, resizable Windows window with drag-and-drop file opening
- GitHub-based update check and install for Windows and macOS releases

## Using PinkDown

1. Launch the application and write in the left pane; the preview on the right updates as you type.
2. Select **Open** or drag a Markdown file into the window to edit an existing document.
3. Select **Save** to write changes to the current file, or **Save as** to choose a new location.
4. Select **Export** to save the document as HTML or PDF. HTML is self-contained and ready to open in a browser; PDF uses a headless Chromium-based browser on your machine (Microsoft Edge or Google Chrome) so the print layout matches the HTML export. Relative images resolve from the open file’s folder. PDF export runs in the background and reports progress in the status bar.
5. Use **Check updates** to compare the installed version against the latest GitHub tag. If a newer release is available, PinkDown asks for confirmation first. Choosing **Update** downloads the release package, verifies its published SHA-256 checksum, and installs it after PinkDown closes (Windows setup EXE; macOS DMG → replace `PinkDown.app`).

The Windows installer installs PinkDown for the current user and registers it as a Markdown handler. Keep the file-association option selected during setup; Windows will open PinkDown's Default Apps page so you can confirm it for `.md` files. Windows requires this system confirmation when another default app is already set.

## Downloads and updates

Official builds are published on the [GitHub Releases page](https://github.com/3xian/PinkDown/releases). Windows is distributed as `pinkdown-windows-x64-setup.exe`; macOS is distributed as a DMG (`pinkdown-macos-arm64.dmg` / `pinkdown-macos-x64.dmg`) containing `PinkDown.app` and an Applications shortcut for Apple Silicon or Intel, with a native multi-resolution icon. Every download includes a SHA-256 checksum file.

See the [Code signing policy](CODE_SIGNING.md) for release provenance, signing
roles, and PinkDown's privacy policy.

The in-app updater downloads `pinkdown-windows-x64-setup.exe` on Windows, or the matching architecture DMG (`pinkdown-macos-arm64.dmg` / `pinkdown-macos-x64.dmg`) on macOS. First-time macOS install is still drag-to-Applications from the DMG; once PinkDown runs from `PinkDown.app`, in-app updates replace that bundle automatically.

GitHub's API limits unauthenticated requests to 60 per hour per IP, which shared VPN or NAT exit addresses can exhaust. If **Check updates** reports a rate-limit error, set a personal access token so PinkDown authenticates against the API (5,000 requests per hour): create a token at GitHub → Settings → Developer settings → Personal access tokens (a classic token with `repo` scope is enough), then set the `GITHUB_TOKEN` or `GH_TOKEN` environment variable for your user and restart PinkDown. If you use the GitHub CLI, `gh auth token` prints the token `gh` already stores.

## Run from source

Install the stable Rust toolchain, then run:

```bash
cargo run --release
```

## Build release binaries

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

Compiled binaries are written to `target/<target>/release/`.

## License

[MIT](LICENSE)
