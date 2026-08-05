//! Export Markdown as self-contained HTML, or as PDF via a system Chromium
//! browser (`--print-to-pdf`) on a background thread.

mod chromium;
mod html;

use std::{
    fs,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
};

use rfd::FileDialog;

use html::HtmlSkin;

#[derive(Clone, Copy)]
pub enum ExportFormat {
    Html,
    Pdf,
}

pub enum ExportPoll {
    Idle,
    Pending,
    Ready(Result<PathBuf, String>),
}

/// Background PDF export job (same pattern as [`crate::update::UpdateChecker`]).
#[derive(Default)]
pub struct ExportJob {
    receiver: Option<Receiver<Result<PathBuf, String>>>,
}

impl ExportJob {
    pub fn is_busy(&self) -> bool {
        self.receiver.is_some()
    }

    pub fn poll(&mut self) -> ExportPoll {
        let Some(receiver) = &self.receiver else {
            return ExportPoll::Idle;
        };
        match receiver.try_recv() {
            Ok(result) => {
                self.receiver = None;
                ExportPoll::Ready(result)
            }
            Err(TryRecvError::Empty) => ExportPoll::Pending,
            Err(TryRecvError::Disconnected) => {
                self.receiver = None;
                ExportPoll::Ready(Err("Export did not complete".into()))
            }
        }
    }

    /// Run PDF export off the UI thread. `path` must already be chosen.
    pub fn start_pdf(
        &mut self,
        path: PathBuf,
        source: String,
        title: String,
        base_dir: Option<PathBuf>,
    ) -> bool {
        if self.receiver.is_some() {
            return false;
        }
        let (sender, receiver) = mpsc::channel();
        self.receiver = Some(receiver);
        thread::spawn(move || {
            let result =
                write_pdf_to(&path, &source, &title, base_dir.as_deref()).map(|()| path);
            let _ = sender.send(result);
        });
        true
    }
}

/// Prompt for a destination path (runs on the calling thread; use before background work).
pub fn pick_destination(title: &str, format: ExportFormat) -> Option<PathBuf> {
    let (filter, ext) = match format {
        ExportFormat::Html => ("HTML", "html"),
        ExportFormat::Pdf => ("PDF", "pdf"),
    };
    let default_name = format!("{title}.{ext}");
    FileDialog::new()
        .add_filter(filter, &[ext])
        .set_file_name(&default_name)
        .save_file()
}

/// Save dialog + write HTML synchronously (fast enough for the UI thread).
pub fn export_html(
    source: &str,
    title: &str,
    base_dir: Option<&Path>,
) -> Result<Option<PathBuf>, String> {
    let Some(path) = pick_destination(title, ExportFormat::Html) else {
        return Ok(None);
    };
    write_html_to(&path, source, title, base_dir)?;
    Ok(Some(path))
}

pub fn write_html_to(
    path: &Path,
    source: &str,
    title: &str,
    base_dir: Option<&Path>,
) -> Result<(), String> {
    let document = html::render_html_document(source, title, HtmlSkin::Screen, base_dir);
    fs::write(path, document).map_err(|error| format!("Could not export HTML: {error}"))
}

/// Write PDF using Chromium headless print. Safe to call from a worker thread.
pub fn write_pdf_to(
    path: &Path,
    source: &str,
    title: &str,
    base_dir: Option<&Path>,
) -> Result<(), String> {
    let stamp = unique_stamp();
    let temp_html = std::env::temp_dir().join(format!("pinkdown-export-{stamp}.html"));
    let temp_pdf = std::env::temp_dir().join(format!("pinkdown-export-{stamp}.pdf"));

    let document = html::render_html_document(source, title, HtmlSkin::Print, base_dir);
    let write_result = (|| {
        fs::write(&temp_html, document)
            .map_err(|error| format!("Could not prepare PDF: {error}"))?;
        chromium::print_html_to_pdf(&temp_html, &temp_pdf)?;
        fs::copy(&temp_pdf, path).map_err(|error| format!("Could not export PDF: {error}"))?;
        Ok(())
    })();

    let _ = fs::remove_file(&temp_html);
    let _ = fs::remove_file(&temp_pdf);
    write_result
}

pub(crate) fn unique_stamp() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{}-{}", std::process::id(), nanos)
}

pub(crate) fn path_to_file_url(path: &Path) -> Result<String, String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| format!("Could not resolve path for export: {error}"))?
            .join(path)
    };

    let absolute = fs::canonicalize(&absolute).unwrap_or(absolute);
    let absolute = strip_windows_verbatim_prefix(&absolute);

    #[cfg(target_os = "windows")]
    {
        let mut path = absolute.to_string_lossy().replace('\\', "/");
        if path.starts_with("//") {
            return Ok(format!("file:{path}"));
        }
        if !path.starts_with('/') {
            path.insert(0, '/');
        }
        // Percent-encode spaces and other characters that break file URLs in Chromium.
        Ok(format!("file://{}", encode_file_url_path(&path)))
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(format!(
            "file://{}",
            encode_file_url_path(&absolute.to_string_lossy())
        ))
    }
}

fn encode_file_url_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for b in path.bytes() {
        match b {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'/'
            | b'-'
            | b'_'
            | b'.'
            | b'~'
            | b':' => out.push(b as char),
            _ => {
                out.push('%');
                const HEX: &[u8; 16] = b"0123456789ABCDEF";
                out.push(HEX[(b >> 4) as usize] as char);
                out.push(HEX[(b & 0xf) as usize] as char);
            }
        }
    }
    out
}

fn strip_windows_verbatim_prefix(path: &Path) -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let s = path.to_string_lossy();
        if let Some(stripped) = s.strip_prefix(r"\\?\") {
            return PathBuf::from(stripped);
        }
    }
    path.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_preserves_ascii_path_chars() {
        assert_eq!(encode_file_url_path("/C:/Users/x/a-b_c.html"), "/C:/Users/x/a-b_c.html");
    }

    #[test]
    fn encode_escapes_spaces() {
        assert_eq!(encode_file_url_path("/C:/My Docs/a.html"), "/C:/My%20Docs/a.html");
    }
}
