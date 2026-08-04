use std::{
    fs,
    io::{self, ErrorKind},
    path::{Path, PathBuf},
};

use chardetng::EncodingDetector;
use encoding_rs::{Encoding, UTF_16BE, UTF_16LE};
use rfd::FileDialog;

const WELCOME: &str = "# Welcome to PinkDown\n\nA calm place for your **ideas**, notes, and writing.\n\n> Write on the left. See it take shape on the right.\n\n## A tiny, beautiful editor\n\n- Open any `.md` file\n- Save as you work\n- Stay focused\n\n```rust\nfn hello() {\n    println!(\"Hello, PinkDown!\");\n}\n```\n\n---\n\nMade with warmth and precision.";

#[derive(Clone, Copy)]
enum TextEncoding {
    Utf8 { bom: bool },
    Utf16Le,
    Utf16Be,
    Legacy(&'static Encoding),
}

impl TextEncoding {
    fn label(self) -> &'static str {
        match self {
            Self::Utf8 { bom: true } => "UTF-8 with BOM",
            Self::Utf8 { bom: false } => "UTF-8",
            Self::Utf16Le => "UTF-16 LE",
            Self::Utf16Be => "UTF-16 BE",
            Self::Legacy(encoding) => encoding.name(),
        }
    }

    fn encode(self, text: &str) -> io::Result<Vec<u8>> {
        match self {
            Self::Utf8 { bom } => {
                let mut bytes = Vec::with_capacity(text.len() + usize::from(bom) * 3);
                if bom {
                    bytes.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
                }
                bytes.extend_from_slice(text.as_bytes());
                Ok(bytes)
            }
            Self::Utf16Le => Ok(encode_utf16(text, u16::to_le_bytes, [0xFF, 0xFE])),
            Self::Utf16Be => Ok(encode_utf16(text, u16::to_be_bytes, [0xFE, 0xFF])),
            Self::Legacy(encoding) => {
                let (bytes, _, had_errors) = encoding.encode(text);
                if had_errors {
                    Err(io::Error::new(
                        ErrorKind::InvalidData,
                        format!(
                            "the edited text contains characters that cannot be represented as {}",
                            encoding.name()
                        ),
                    ))
                } else {
                    Ok(bytes.into_owned())
                }
            }
        }
    }
}

fn encode_utf16(text: &str, byte_order: fn(u16) -> [u8; 2], bom: [u8; 2]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(2 + text.len() * 2);
    bytes.extend_from_slice(&bom);
    for code_unit in text.encode_utf16() {
        bytes.extend_from_slice(&byte_order(code_unit));
    }
    bytes
}

pub struct Document {
    pub text: String,
    saved_text: String,
    path: Option<PathBuf>,
    encoding: TextEncoding,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            text: WELCOME.to_owned(),
            saved_text: WELCOME.to_owned(),
            path: None,
            encoding: TextEncoding::Utf8 { bom: false },
        }
    }
}

impl Document {
    pub fn load(path: PathBuf) -> Result<Self, String> {
        let bytes = fs::read(&path).map_err(|error| format!("Could not open file: {error}"))?;
        let (text, encoding) = decode(&bytes)
            .map_err(|error| format!("Could not decode {}: {error}", display_name(&path)))?;
        Ok(Self {
            saved_text: text.clone(),
            text,
            path: Some(path),
            encoding,
        })
    }

    pub fn is_dirty(&self) -> bool {
        self.text != self.saved_text
    }

    pub fn display_name(&self) -> String {
        self.path
            .as_deref()
            .map_or_else(|| "Untitled".to_owned(), display_name)
    }

    /// Window title bar / taskbar text; `None` while no file is open.
    /// A dirty document gets a trailing `*` marker.
    pub fn window_title(&self) -> Option<String> {
        let mut title = display_name(self.path.as_deref()?);
        if self.is_dirty() {
            title.push('*');
        }
        Some(title)
    }

    pub fn encoding_label(&self) -> &'static str {
        self.encoding.label()
    }

    /// Saves the document and returns `false` only when the user cancels the dialog.
    pub fn save(&mut self, force_dialog: bool) -> Result<bool, String> {
        let path = if force_dialog || self.path.is_none() {
            FileDialog::new()
                .add_filter("Markdown", &["md"])
                .set_file_name("untitled.md")
                .save_file()
        } else {
            self.path.clone()
        };
        let Some(path) = path else {
            return Ok(false);
        };

        let bytes = self
            .encoding
            .encode(&self.text)
            .map_err(|error| format!("Could not encode file: {error}"))?;
        fs::write(&path, bytes).map_err(|error| format!("Could not save file: {error}"))?;
        self.saved_text.clone_from(&self.text);
        self.path = Some(path);
        Ok(true)
    }
}

pub fn pick_markdown_file() -> Option<PathBuf> {
    FileDialog::new()
        .add_filter("Markdown", &["md", "markdown", "mdx", "txt"])
        .pick_file()
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Untitled")
        .to_owned()
}

fn decode(bytes: &[u8]) -> io::Result<(String, TextEncoding)> {
    if let Some(content) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        return decode_with(UTF_16LE, content, TextEncoding::Utf16Le);
    }
    if let Some(content) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        return decode_with(UTF_16BE, content, TextEncoding::Utf16Be);
    }

    let (content, bom) = bytes
        .strip_prefix(&[0xEF, 0xBB, 0xBF])
        .map_or((bytes, false), |content| (content, true));
    if let Ok(text) = std::str::from_utf8(content) {
        return Ok((text.to_owned(), TextEncoding::Utf8 { bom }));
    }

    let mut detector = EncodingDetector::new();
    detector.feed(bytes, true);
    let encoding = detector.guess(None, false);
    decode_with(encoding, bytes, TextEncoding::Legacy(encoding))
}

fn decode_with(
    encoding: &'static Encoding,
    bytes: &[u8],
    kind: TextEncoding,
) -> io::Result<(String, TextEncoding)> {
    let (text, had_errors) = encoding.decode_without_bom_handling(bytes);
    if had_errors {
        Err(io::Error::new(
            ErrorKind::InvalidData,
            format!(
                "invalid {} byte sequence; the file was not opened",
                kind.label()
            ),
        ))
    } else {
        Ok((text.into_owned(), kind))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_little_endian_round_trips_with_bom() {
        let source = "Hello, 世界 👋";
        let bytes = TextEncoding::Utf16Le.encode(source).unwrap();
        let (decoded, encoding) = decode(&bytes).unwrap();
        assert_eq!(decoded, source);
        assert_eq!(encoding.label(), "UTF-16 LE");
    }

    #[test]
    fn loads_utf16_file_through_the_document_boundary() {
        let path =
            std::env::temp_dir().join(format!("pinkdown-document-{}-utf16.md", std::process::id()));
        let source = "Loaded through Document::load 世界";
        fs::write(&path, TextEncoding::Utf16Le.encode(source).unwrap()).unwrap();

        let document = Document::load(path.clone()).unwrap();

        let _ = fs::remove_file(path);
        assert_eq!(document.text, source);
        assert_eq!(document.encoding_label(), "UTF-16 LE");
        assert!(!document.is_dirty());
    }

    #[test]
    fn utf16_big_endian_round_trips_with_bom() {
        let source = "PinkDown 🌹";
        let bytes = TextEncoding::Utf16Be.encode(source).unwrap();
        let (decoded, encoding) = decode(&bytes).unwrap();
        assert_eq!(decoded, source);
        assert_eq!(encoding.label(), "UTF-16 BE");
    }

    #[test]
    fn save_preserves_the_loaded_utf16_encoding() {
        let path =
            std::env::temp_dir().join(format!("pinkdown-document-{}-save.md", std::process::id()));
        let original = "before";
        fs::write(
            &path,
            TextEncoding::Utf16Le
                .encode(original)
                .expect("encode fixture"),
        )
        .expect("write fixture");
        let mut document = Document::load(path.clone()).expect("load fixture");
        document.text = "after 世界".to_owned();

        assert!(document.save(false).expect("save document"));
        let saved = fs::read(&path).expect("read saved file");
        let _ = fs::remove_file(path);
        let (text, encoding) = decode(&saved).expect("decode saved file");
        assert_eq!(text, "after 世界");
        assert_eq!(encoding.label(), "UTF-16 LE");
    }

    #[test]
    fn utf8_bom_is_preserved() {
        let source = "hello";
        let encoding = TextEncoding::Utf8 { bom: true };
        let bytes = encoding.encode(source).unwrap();
        assert!(bytes.starts_with(&[0xEF, 0xBB, 0xBF]));
        let (decoded, detected) = decode(&bytes).unwrap();
        assert_eq!(decoded, source);
        assert_eq!(detected.label(), "UTF-8 with BOM");
    }

    #[test]
    fn untitled_document_becomes_dirty_after_edit() {
        let mut document = Document::default();
        assert!(!document.is_dirty());
        document.text.push('!');
        assert!(document.is_dirty());
    }

    #[test]
    fn window_title_reflects_path_and_dirty_state() {
        let pid = std::process::id();
        let title_path =
            std::env::temp_dir().join(format!("pinkdown-document-{pid}-title.md"));
        fs::write(&title_path, "hello").unwrap();
        let document = Document::load(title_path.clone()).unwrap();
        let expected = format!("pinkdown-document-{pid}-title.md");
        assert_eq!(document.window_title(), Some(expected));
        let _ = fs::remove_file(title_path);

        let untitled = Document::default();
        assert_eq!(untitled.window_title(), None);

        let dirty_path =
            std::env::temp_dir().join(format!("pinkdown-document-{pid}-dirty.md"));
        fs::write(&dirty_path, "hello").unwrap();
        let mut dirty = Document::load(dirty_path.clone()).unwrap();
        dirty.text.push('!');
        assert_eq!(
            dirty.window_title(),
            Some(format!("pinkdown-document-{pid}-dirty.md*"))
        );
        let _ = fs::remove_file(dirty_path);
    }
}
