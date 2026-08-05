//! Locate a Chromium-based browser and print HTML to PDF headlessly.

use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use super::{path_to_file_url, unique_stamp};

/// Render is done by the caller; this only drives headless Chromium print.
pub fn print_html_to_pdf(html_path: &Path, pdf_path: &Path) -> Result<(), String> {
    let browser = find_chromium_browser().ok_or_else(no_browser_message)?;

    if pdf_path.exists() {
        let _ = fs::remove_file(pdf_path);
    }

    let stamp = unique_stamp();
    let profile_dir = std::env::temp_dir().join(format!("pinkdown-chrome-{stamp}"));
    fs::create_dir_all(&profile_dir)
        .map_err(|error| format!("Could not prepare PDF browser profile: {error}"))?;

    let result = print_with_profile(&browser, html_path, pdf_path, &profile_dir);
    let _ = fs::remove_dir_all(&profile_dir);
    result
}

fn print_with_profile(
    browser: &Path,
    html_path: &Path,
    pdf_path: &Path,
    profile_dir: &Path,
) -> Result<(), String> {
    let html_url = path_to_file_url(html_path)?;
    let user_data = format!("--user-data-dir={}", profile_dir.display());
    let print_flag = format!("--print-to-pdf={}", pdf_path.display());

    let mut child = Command::new(browser)
        .args([
            "--headless=new",
            user_data.as_str(),
            "--disable-gpu",
            "--disable-extensions",
            "--no-first-run",
            "--no-default-browser-check",
            "--no-pdf-header-footer",
            "--allow-file-access-from-files",
            print_flag.as_str(),
            html_url.as_str(),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("Could not start browser for PDF export: {error}"))?;

    let browser_status = wait_with_timeout(&mut child, Duration::from_secs(60));

    match validate_pdf(pdf_path, Duration::from_secs(2)) {
        Ok(()) => Ok(()),
        Err(pdf_error) => match browser_status {
            Ok(()) => Err(pdf_error),
            Err(browser_error) => Err(format!(
                "Could not export PDF: browser print failed ({browser_error}). Try exporting HTML instead."
            )),
        },
    }
}

fn no_browser_message() -> String {
    "Could not export PDF: no Chromium-based browser found (install Microsoft Edge or Google Chrome, or export as HTML and print to PDF from the browser).".into()
}

/// Wait until the PDF exists with a stable non-zero size and a valid header.
fn validate_pdf(pdf_path: &Path, timeout: Duration) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    let mut last_len: Option<u64> = None;

    while Instant::now() < deadline {
        if pdf_path.is_file() {
            let len = fs::metadata(pdf_path).map(|m| m.len()).unwrap_or(0);
            if len > 0 {
                if last_len == Some(len) {
                    return check_pdf_header(pdf_path);
                }
                last_len = Some(len);
            }
        }
        thread::sleep(Duration::from_millis(50));
    }

    if pdf_path.is_file() {
        let len = fs::metadata(pdf_path).map(|m| m.len()).unwrap_or(0);
        if len > 0 {
            return check_pdf_header(pdf_path);
        }
    }
    Err("Could not export PDF: browser did not produce a PDF file.".into())
}

fn check_pdf_header(pdf_path: &Path) -> Result<(), String> {
    let mut file = fs::File::open(pdf_path)
        .map_err(|error| format!("Could not read exported PDF: {error}"))?;
    let mut header = [0u8; 5];
    let n = file
        .read(&mut header)
        .map_err(|error| format!("Could not read exported PDF: {error}"))?;
    if n >= 4 && header.starts_with(b"%PDF") {
        Ok(())
    } else {
        Err("Could not export PDF: browser wrote an invalid PDF.".into())
    }
}

fn wait_with_timeout(child: &mut std::process::Child, timeout: Duration) -> Result<(), String> {
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(status)) => {
                return Err(format!("exit code {}", status.code().unwrap_or(-1)));
            }
            Ok(None) if start.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("timed out".into());
            }
            Ok(None) => thread::sleep(Duration::from_millis(40)),
            Err(error) => return Err(error.to_string()),
        }
    }
}

/// Locate Edge / Chrome / Chromium for headless printing.
pub fn find_chromium_browser() -> Option<PathBuf> {
    for candidate in browser_candidates() {
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    for name in browser_names_on_path() {
        if let Some(path) = which(name) {
            return Some(path);
        }
    }
    None
}

fn browser_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    #[cfg(target_os = "windows")]
    {
        let mut roots = Vec::new();
        for key in ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"] {
            if let Some(value) = std::env::var_os(key) {
                roots.push(PathBuf::from(value));
            }
        }
        roots.push(PathBuf::from(r"C:\Program Files"));
        roots.push(PathBuf::from(r"C:\Program Files (x86)"));

        for root in roots {
            paths.push(root.join(r"Microsoft\Edge\Application\msedge.exe"));
            paths.push(root.join(r"Google\Chrome\Application\chrome.exe"));
            paths.push(root.join(r"Chromium\Application\chrome.exe"));
        }
    }

    #[cfg(target_os = "macos")]
    {
        paths.extend([
            PathBuf::from("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"),
            PathBuf::from("/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge"),
            PathBuf::from("/Applications/Chromium.app/Contents/MacOS/Chromium"),
            PathBuf::from(
                "/Applications/Google Chrome Canary.app/Contents/MacOS/Google Chrome Canary",
            ),
        ]);
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        paths.extend([
            PathBuf::from("/usr/bin/google-chrome-stable"),
            PathBuf::from("/usr/bin/google-chrome"),
            PathBuf::from("/usr/bin/chromium"),
            PathBuf::from("/usr/bin/chromium-browser"),
            PathBuf::from("/usr/bin/microsoft-edge"),
            PathBuf::from("/usr/bin/microsoft-edge-stable"),
            PathBuf::from("/snap/bin/chromium"),
        ]);
    }

    paths
}

fn browser_names_on_path() -> &'static [&'static str] {
    #[cfg(target_os = "windows")]
    {
        &["msedge", "chrome", "chromium"]
    }
    #[cfg(target_os = "macos")]
    {
        &["google-chrome", "chromium", "microsoft-edge"]
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        &[
            "google-chrome-stable",
            "google-chrome",
            "chromium",
            "chromium-browser",
            "microsoft-edge",
            "microsoft-edge-stable",
        ]
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", unix)))]
    {
        &[]
    }
}

fn which(name: &str) -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    let output = Command::new("where.exe").arg(name).output().ok()?;
    #[cfg(not(target_os = "windows"))]
    let output = Command::new("which").arg(name).output().ok()?;

    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.lines().next()?.trim();
    if line.is_empty() {
        return None;
    }
    let path = PathBuf::from(line);
    path.is_file().then_some(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::{html, unique_stamp, write_pdf_to};

    #[test]
    fn file_url_uses_forward_slashes() {
        let path = std::env::temp_dir().join(format!(
            "pinkdown-url-{}-{}.html",
            std::process::id(),
            unique_stamp()
        ));
        fs::write(&path, "<html></html>").unwrap();
        let url = path_to_file_url(&path).unwrap();
        let _ = fs::remove_file(&path);
        assert!(url.starts_with("file:"));
        assert!(
            !url.contains('\\'),
            "url should not contain backslashes: {url}"
        );
    }

    #[test]
    fn pdf_export_writes_a_non_empty_file() {
        let Some(_) = find_chromium_browser() else {
            eprintln!("skipping pdf_export_writes_a_non_empty_file: no Chromium browser installed");
            return;
        };

        let path = std::env::temp_dir().join(format!(
            "pinkdown-export-{}-{}.pdf",
            std::process::id(),
            unique_stamp()
        ));
        write_pdf_to(
            &path,
            "# Hello\n\n世界, **PinkDown**.\n\n- a\n- b\n\n| col | val |\n| --- | --- |\n| x | y |\n",
            "Test",
            None,
        )
        .expect("pdf export");
        let bytes = fs::read(&path).expect("read pdf");
        let _ = fs::remove_file(&path);
        assert!(bytes.starts_with(b"%PDF"));
        assert!(bytes.len() > 500);
    }

    #[test]
    fn print_html_uses_print_skin() {
        // Smoke: ensure the HTML helper still builds print docs for the PDF path.
        let html = html::render_html_document("# T", "T", html::HtmlSkin::Print, None);
        assert!(html.contains("@page"));
    }
}
