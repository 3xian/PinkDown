use eframe::egui::{self, RichText};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};

use crate::theme::{self, BASE, HIGHLIGHT_LOW, MUTED};

pub fn panel(ui: &mut egui::Ui, source: &str, cache: &mut CommonMarkCache) {
    egui::Frame::new()
        .fill(BASE)
        .stroke(egui::Stroke::new(1.0, HIGHLIGHT_LOW))
        .corner_radius(egui::CornerRadius::same(12))
        .inner_margin(egui::Margin::symmetric(18, 16))
        .show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            ui.horizontal(|ui| {
                ui.label(RichText::new("PREVIEW").size(11.0).strong().color(MUTED));
                ui.label(RichText::new("COMMONMARK").size(10.0).color(MUTED));
            });
            ui.add_space(8.0);
            egui::ScrollArea::vertical()
                .id_salt("preview-scroll")
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.set_width(ui.available_width().max(1.0));
                    ui.scope(|ui| {
                        theme::configure_preview(ui);
                        CommonMarkViewer::new().show(ui, cache, source);
                    });
                    ui.add_space(24.0);
                });
        });
}

#[cfg(test)]
mod tests {
    use pulldown_cmark::{Event, Options, Parser, Tag};

    fn events(source: &str) -> Vec<Event<'_>> {
        Parser::new_ext(source, Options::ENABLE_TABLES).collect()
    }

    #[test]
    fn parses_wide_chinese_table_without_splitting_inline_code() {
        let source = "| 门禁 | 人工必须确认 | Agent 才可以 |\n\
                      |---|---|---|\n\
                      | G1 | `PLAY\\|Bxxx` 与 a \\| b | `new_video.py` |";
        let parsed = events(source);

        assert_eq!(
            parsed
                .iter()
                .filter(|event| matches!(event, Event::Start(Tag::Table(_))))
                .count(),
            1
        );
        assert!(parsed
            .iter()
            .any(|event| { matches!(event, Event::Code(code) if code.as_ref() == "PLAY|Bxxx") }));
        assert_eq!(
            parsed
                .iter()
                .filter(|event| matches!(event, Event::Start(Tag::TableCell)))
                .count(),
            6
        );
    }

    #[test]
    fn resolves_reference_links_across_headings() {
        let parsed = events("[link][target]\n\n# Heading\n\n[target]: https://example.com");

        assert!(parsed.iter().any(|event| {
            matches!(
                event,
                Event::Start(Tag::Link { dest_url, .. }) if dest_url.as_ref() == "https://example.com"
            )
        }));
    }

    #[test]
    fn preserves_fenced_code_block_inside_ordered_list() {
        let source = "1. 把新导出的 `视频号动态数据明细*.csv` 放在仓库根目录，运行：\n\n   \
                      ```powershell\n   python scripts/analyze_wechat_export.py\n   ```";
        let parsed = events(source);

        assert!(parsed
            .iter()
            .any(|event| { matches!(event, Event::Start(Tag::List(Some(1)))) }));
        assert!(parsed.iter().any(|event| {
            matches!(
                event,
                Event::Start(Tag::CodeBlock(pulldown_cmark::CodeBlockKind::Fenced(language)))
                    if language.as_ref() == "powershell"
            )
        }));
        assert!(parsed.iter().any(|event| {
            matches!(event, Event::Text(text) if text.as_ref().contains("analyze_wechat_export.py"))
        }));
    }
}
