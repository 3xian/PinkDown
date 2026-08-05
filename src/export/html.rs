//! Markdown → self-contained HTML (screen and print skins).

use std::path::Path;

use pulldown_cmark::{CowStr, Event, Options, Parser, Tag};

use super::path_to_file_url;

#[derive(Clone, Copy)]
pub enum HtmlSkin {
    /// Interactive / browser viewing (respects light/dark preference).
    Screen,
    /// Headless print target: fixed light palette + page rules for clean PDFs.
    Print,
}

fn markdown_options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES
}

pub fn render_html_document(
    source: &str,
    title: &str,
    skin: HtmlSkin,
    base_dir: Option<&Path>,
) -> String {
    let mut body = String::new();
    let parser = Parser::new_ext(source, markdown_options());
    match base_dir {
        Some(base) => {
            let events = parser.map(|event| rewrite_relative_urls(event, base));
            pulldown_cmark::html::push_html(&mut body, events);
        }
        None => pulldown_cmark::html::push_html(&mut body, parser),
    }

    let escaped_title = escape_html(title);
    let skin_css = css_for(skin);

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{escaped_title}</title>
<style>
{skin_css}
</style>
</head>
<body>
<main>
  {body}
</main>
</body>
</html>
"#
    )
}

/// Point relative image/link destinations at the document directory so exports
/// (especially PDF from a temp HTML file) still resolve local assets.
fn rewrite_relative_urls<'a>(event: Event<'a>, base_dir: &Path) -> Event<'a> {
    match event {
        Event::Start(Tag::Image {
            link_type,
            dest_url,
            title,
            id,
        }) => Event::Start(Tag::Image {
            link_type,
            dest_url: resolve_resource_url(&dest_url, base_dir),
            title,
            id,
        }),
        Event::Start(Tag::Link {
            link_type,
            dest_url,
            title,
            id,
        }) => Event::Start(Tag::Link {
            link_type,
            dest_url: resolve_resource_url(&dest_url, base_dir),
            title,
            id,
        }),
        other => other,
    }
}

fn resolve_resource_url(url: &str, base_dir: &Path) -> CowStr<'static> {
    if url.is_empty()
        || url.starts_with('#')
        || url.starts_with("data:")
        || url.starts_with("mailto:")
        || url.contains("://")
    {
        return CowStr::from(url.to_owned());
    }

    let path = Path::new(url);
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    };
    match path_to_file_url(&absolute) {
        Ok(file_url) => CowStr::from(file_url),
        Err(_) => CowStr::from(url.to_owned()),
    }
}

fn css_for(skin: HtmlSkin) -> String {
    match skin {
        HtmlSkin::Screen => format!("{SCREEN_VARS}{SHARED_CSS}{SCREEN_EXTRA}"),
        HtmlSkin::Print => format!("{PRINT_VARS}{SHARED_CSS}{PRINT_EXTRA}"),
    }
}

/// Shared layout rules (colors come from each skin's `:root`).
const SHARED_CSS: &str = r#"
  * { box-sizing: border-box; }
  body {
    margin: 0;
    font: 16px/1.65 system-ui, -apple-system, "Segoe UI", "PingFang SC",
          "Microsoft YaHei", "Noto Sans CJK SC", sans-serif;
    color: var(--fg);
    background: var(--bg);
  }
  main {
    max-width: 44rem;
    margin: 0 auto;
    padding: 2.5rem 1.5rem 4rem;
  }
  h1, h2, h3, h4, h5, h6 {
    line-height: 1.25;
    margin: 1.6em 0 0.6em;
    font-weight: 650;
  }
  h1 { font-size: 1.9rem; margin-top: 0; }
  h2 { font-size: 1.45rem; }
  h3 { font-size: 1.2rem; }
  p, ul, ol, pre, blockquote, table { margin: 0 0 1em; }
  a { color: var(--link); }
  code {
    font-family: ui-monospace, "Cascadia Code", "SF Mono", Menlo, Consolas, monospace;
    font-size: 0.9em;
    background: var(--code-bg);
    padding: 0.12em 0.35em;
    border-radius: 4px;
  }
  pre {
    background: var(--code-bg);
    color: var(--pre-fg);
    padding: 1rem 1.1rem;
    border-radius: 10px;
    overflow-x: auto;
  }
  pre code {
    background: transparent;
    padding: 0;
    font-size: 0.88em;
  }
  blockquote {
    margin-left: 0;
    padding: 0.15em 0 0.15em 1em;
    border-left: 3px solid var(--quote);
    color: var(--muted);
  }
  hr {
    border: 0;
    border-top: 1px solid var(--border);
    margin: 2em 0;
  }
  th, td {
    border: 1px solid var(--border);
    padding: 0.45em 0.7em;
    text-align: left;
  }
  th { background: var(--code-bg); }
  img { max-width: 100%; height: auto; }
"#;

const SCREEN_VARS: &str = r#"
  :root {
    color-scheme: light dark;
    --bg: #faf8f9;
    --fg: #2a2430;
    --muted: #6e6a86;
    --border: #e4dce3;
    --code-bg: #f0eaf0;
    --quote: #907aa9;
    --link: #286983;
    --pre-fg: #575279;
  }
  @media (prefers-color-scheme: dark) {
    :root {
      --bg: #191724;
      --fg: #e0def4;
      --muted: #908caa;
      --border: #403d52;
      --code-bg: #26233a;
      --quote: #c4a7e7;
      --link: #9ccfd8;
      --pre-fg: #e0def4;
    }
  }
"#;

const SCREEN_EXTRA: &str = r#"
  table {
    border-collapse: collapse;
    width: 100%;
    display: block;
    overflow-x: auto;
  }
"#;

const PRINT_VARS: &str = r#"
  :root {
    color-scheme: light;
    --bg: #ffffff;
    --fg: #2a2430;
    --muted: #6e6a86;
    --border: #d4cdd3;
    --code-bg: #f3eef3;
    --quote: #907aa9;
    --link: #286983;
    --pre-fg: #3e3a4a;
  }
  @page {
    size: A4;
    margin: 16mm 14mm;
  }
"#;

const PRINT_EXTRA: &str = r#"
  html, body {
    background: #ffffff !important;
  }
  main {
    max-width: none;
    padding: 0;
  }
  table {
    border-collapse: collapse;
    width: 100%;
  }
  pre, blockquote, table, img {
    break-inside: avoid;
    page-break-inside: avoid;
  }
  h1, h2, h3, h4, h5, h6 {
    break-after: avoid;
    page-break-after: avoid;
  }
  a {
    text-decoration: none;
  }
"#;

fn escape_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_export_contains_rendered_heading() {
        let html =
            render_html_document("# Hello\n\nParagraph with **bold**.", "Doc", HtmlSkin::Screen, None);
        assert!(html.contains("<h1>Hello</h1>"));
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<title>Doc</title>"));
        assert!(html.contains("prefers-color-scheme: dark"));
    }

    #[test]
    fn print_skin_is_light_and_paged() {
        let html = render_html_document("# Title", "Doc", HtmlSkin::Print, None);
        assert!(html.contains("@page"));
        assert!(html.contains("color-scheme: light"));
        assert!(!html.contains("prefers-color-scheme: dark"));
    }

    #[test]
    fn escapes_html_in_title() {
        let html = render_html_document("# x", "A <B> & C", HtmlSkin::Screen, None);
        assert!(html.contains("<title>A &lt;B&gt; &amp; C</title>"));
    }

    #[test]
    fn rewrites_relative_image_to_file_url() {
        let base = std::env::temp_dir();
        let html = render_html_document(
            "![alt](./photo.png)",
            "Doc",
            HtmlSkin::Screen,
            Some(&base),
        );
        assert!(
            html.contains("file:") && html.contains("photo.png"),
            "expected file URL for relative image, got: {html}"
        );
        assert!(!html.contains("src=\"./photo.png\""));
    }

    #[test]
    fn leaves_http_links_alone() {
        let base = std::env::temp_dir();
        let html = render_html_document(
            "[x](https://example.com/a.png)",
            "Doc",
            HtmlSkin::Screen,
            Some(&base),
        );
        assert!(html.contains("https://example.com/a.png"));
    }
}
