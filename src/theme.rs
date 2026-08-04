use std::borrow::Cow;
use std::fs;
use std::sync::Arc;

use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, FontId};

pub const BASE: Color32 = Color32::from_rgb(25, 23, 36);
pub const SURFACE: Color32 = Color32::from_rgb(31, 29, 46);
pub const OVERLAY: Color32 = Color32::from_rgb(38, 35, 58);
pub const MUTED: Color32 = Color32::from_rgb(110, 106, 134);
pub const SUBTLE: Color32 = Color32::from_rgb(144, 140, 170);
pub const TEXT: Color32 = Color32::from_rgb(224, 222, 244);
pub const LOVE: Color32 = Color32::from_rgb(235, 111, 146);
pub const GOLD: Color32 = Color32::from_rgb(246, 193, 119);
pub const ROSE: Color32 = Color32::from_rgb(235, 188, 186);
pub const PINE: Color32 = Color32::from_rgb(49, 116, 143);
pub const FOAM: Color32 = Color32::from_rgb(156, 207, 216);
pub const IRIS: Color32 = Color32::from_rgb(196, 167, 231);
pub const HIGHLIGHT_LOW: Color32 = Color32::from_rgb(33, 32, 46);
pub const HIGHLIGHT_MED: Color32 = Color32::from_rgb(64, 61, 82);
pub const HIGHLIGHT_HIGH: Color32 = Color32::from_rgb(82, 79, 103);
pub const CONTENT_FONT_SIZE: f32 = 13.0;

pub fn icon_data() -> egui::IconData {
    #[cfg(target_os = "macos")]
    const ICON_BYTES: &[u8] = include_bytes!("../assets/pinkdown-macos-icon.png");
    #[cfg(not(target_os = "macos"))]
    const ICON_BYTES: &[u8] = include_bytes!("../assets/pinkdown-icon.png");

    let image = image::load_from_memory_with_format(ICON_BYTES, image::ImageFormat::Png)
        .expect("decode the embedded PinkDown icon")
        .resize_exact(64, 64, image::imageops::FilterType::Lanczos3)
        .into_rgba8();
    egui::IconData {
        rgba: image.into_raw(),
        width: 64_u32,
        height: 64_u32,
    }
}

pub fn configure(ctx: &egui::Context) {
    configure_fonts(ctx);

    let mut style = (*ctx.style()).clone();
    style.visuals = egui::Visuals::dark();
    style.visuals.panel_fill = Color32::TRANSPARENT;
    style.visuals.window_fill = SURFACE;
    style.visuals.extreme_bg_color = BASE;
    style.visuals.text_edit_bg_color = Some(SURFACE);
    style.visuals.code_bg_color = OVERLAY;
    style.visuals.faint_bg_color = OVERLAY;
    style.visuals.weak_text_color = Some(SUBTLE);
    style.visuals.widgets.noninteractive.bg_fill = SURFACE;
    style.visuals.widgets.noninteractive.weak_bg_fill = BASE;
    style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0_f32, TEXT);
    style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0_f32, HIGHLIGHT_MED);
    style.visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(8);
    style.visuals.widgets.inactive.bg_fill = OVERLAY;
    style.visuals.widgets.inactive.weak_bg_fill = OVERLAY;
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0_f32, HIGHLIGHT_MED);
    style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0_f32, SUBTLE);
    style.visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(8);
    style.visuals.widgets.hovered.bg_fill = HIGHLIGHT_MED;
    style.visuals.widgets.hovered.weak_bg_fill = HIGHLIGHT_MED;
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0_f32, HIGHLIGHT_HIGH);
    style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0_f32, FOAM);
    style.visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(8);
    style.visuals.widgets.active.bg_fill = HIGHLIGHT_HIGH;
    style.visuals.widgets.active.weak_bg_fill = HIGHLIGHT_HIGH;
    style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0_f32, IRIS);
    style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0_f32, ROSE);
    style.visuals.widgets.active.corner_radius = egui::CornerRadius::same(8);
    style.visuals.widgets.open = style.visuals.widgets.active;
    style.visuals.selection.bg_fill = PINE;
    style.visuals.selection.stroke.color = FOAM;
    style.visuals.hyperlink_color = FOAM;
    style.visuals.warn_fg_color = GOLD;
    style.visuals.error_fg_color = LOVE;
    style.visuals.window_corner_radius = egui::CornerRadius::same(16);
    style.visuals.window_shadow = egui::Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: Color32::from_black_alpha(100),
    };
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 7.0);
    style.text_styles.insert(
        egui::TextStyle::Body,
        FontId::new(15.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Monospace,
        FontId::new(CONTENT_FONT_SIZE, FontFamily::Monospace),
    );
    style.url_in_tooltip = true;
    ctx.set_style(style);
}

pub fn configure_preview(ui: &mut egui::Ui) {
    let style = ui.style_mut();

    // Code blocks sit above the preview's base background; all semantic colors
    // continue to come from the single global Rosé Pine palette above.
    style.visuals.extreme_bg_color = SURFACE;
    style.visuals.code_bg_color = HIGHLIGHT_MED;
    style.visuals.faint_bg_color = HIGHLIGHT_LOW;
    style.visuals.widgets.open.fg_stroke.color = IRIS;
    style.text_styles.insert(
        egui::TextStyle::Body,
        FontId::new(CONTENT_FONT_SIZE, FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Heading,
        FontId::new(18.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Monospace,
        FontId::new(CONTENT_FONT_SIZE, FontFamily::Monospace),
    );
    style.text_styles.insert(
        egui::TextStyle::Small,
        FontId::new(12.0, FontFamily::Proportional),
    );
    for (name, size) in [
        ("Heading1", 26.0),
        ("Heading2", 21.0),
        ("Heading3", 17.0),
        ("Heading4", 16.0),
        ("Heading5", 15.0),
        ("Heading6", 14.0),
    ] {
        style.text_styles.insert(
            egui::TextStyle::Name(name.into()),
            FontId::new(size, FontFamily::Proportional),
        );
    }
    style.spacing.item_spacing.y = 6.0;
    style.spacing.indent = 20.0;
    style.wrap_mode = Some(egui::TextWrapMode::Wrap);
}

/// Extra vertical space between text rows, as a fraction of the font size.
///
/// egui derives the height of every text row from the first font in the
/// family (`epaint::Font::row_height` = `ascent - descent + line_gap`) and
/// exposes no line-spacing knob, so we grow the font's own `lineGap`
/// metric in the loaded bytes instead. That lifts the editor and the
/// preview alike, since both render through the same font families.
///
/// The effect is deliberately app-wide: every text row — editor, preview,
/// buttons, dialogs — grows by the same amount (~2pt at 13pt text), so the
/// whole UI breathes uniformly. Scoping it to the editor and preview alone
/// would require a dedicated patched font family for those surfaces.
const EXTRA_LINE_GAP_EM: f32 = 0.15;

fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    for data in fonts.font_data.values_mut() {
        // `FontDefinitions::default()` builds each font in a fresh, unshared `Arc`
        // today; if egui ever deduplicates them, skip instead of panicking — the
        // worst case is line spacing silently reverting to the font default.
        if let Some(data) = Arc::get_mut(data) {
            data.font = Cow::Owned(patch_line_gap(data.font.to_vec(), EXTRA_LINE_GAP_EM));
        }
    }

    if let Some(bytes) = font_candidates()
        .iter()
        .find_map(|path| fs::read(path).ok())
    {
        let name = "system-cjk".to_owned();
        fonts
            .font_data
            .insert(name.clone(), FontData::from_owned(bytes).into());
        fonts
            .families
            .get_mut(&FontFamily::Proportional)
            .expect("default proportional family")
            .push(name.clone());
        fonts
            .families
            .get_mut(&FontFamily::Monospace)
            .expect("default monospace family")
            .push(name);
    }

    ctx.set_fonts(fonts);
}

/// Grow the `lineGap` horizontal metric (in font units) inside `sfnt` font
/// bytes, in both `hhea` and `OS/2` (`sTypoLineGap`). ttf-parser — which
/// ab_glyph sits on — reads whichever of the two the font's
/// `USE_TYPO_METRICS` flag selects, so both are bumped.
///
/// The delta is `extra_em` × the font's line height (`ascender − descender`,
/// chosen the same way ttf-parser picks metrics), so text rows grow by
/// exactly `extra_em` × font size.
///
/// The table-directory checksums and `head.checkSumAdjustment` are left
/// stale on purpose: ttf-parser never validates sfnt checksums, and the
/// patched bytes only ever feed egui's own font pipeline. Recompute them
/// if the bytes ever reach a checksum-validating consumer.
fn patch_line_gap(mut bytes: Vec<u8>, extra_em: f32) -> Vec<u8> {
    if bytes.len() < 12 {
        return bytes;
    }
    let num_tables = u16::from_be_bytes([bytes[4], bytes[5]]) as usize;
    if bytes.len() < 12 + 16 * num_tables {
        return bytes;
    }
    let mut hhea = None;
    let mut os2 = None;
    for i in 0..num_tables {
        let record = 12 + 16 * i;
        let tag = &bytes[record..record + 4];
        let offset = u32::from_be_bytes([
            bytes[record + 8],
            bytes[record + 9],
            bytes[record + 10],
            bytes[record + 11],
        ]) as usize;
        match tag {
            b"hhea" => hhea = Some(offset),
            b"OS/2" => os2 = Some(offset),
            _ => {}
        }
    }
    let Some(hhea) = hhea else {
        return bytes;
    };
    let Some(height) = line_height_units(&bytes, hhea, os2) else {
        return bytes;
    };
    let delta = (extra_em * height).round() as i16;
    if delta == 0 {
        return bytes;
    }
    bump_i16(&mut bytes, hhea + 8, delta); // hhea.lineGap
    if let Some(os2) = os2 {
        bump_i16(&mut bytes, os2 + 74, delta); // OS/2.sTypoLineGap
    }
    bytes
}

/// The font's line height (`ascender − descender`) in font units, selected
/// the same way ttf-parser picks metrics: OS/2 typo metrics when
/// `USE_TYPO_METRICS` (fsSelection bit 7) is set, otherwise hhea — falling
/// back to OS/2 typo metrics when hhea's ascender or descender is zero.
fn line_height_units(bytes: &[u8], hhea: usize, os2: Option<usize>) -> Option<f32> {
    if hhea + 8 > bytes.len() {
        return None;
    }
    let hhea_asc = i16::from_be_bytes([bytes[hhea + 4], bytes[hhea + 5]]);
    let hhea_desc = i16::from_be_bytes([bytes[hhea + 6], bytes[hhea + 7]]);
    let os2 = os2.filter(|&o| o + 74 <= bytes.len());
    let use_typo = os2
        .is_some_and(|o| i16::from_be_bytes([bytes[o + 62], bytes[o + 63]]) & 0x0080 != 0);
    let (asc, desc) = if use_typo || hhea_asc == 0 || hhea_desc == 0 {
        let os2 = os2?;
        (
            i16::from_be_bytes([bytes[os2 + 68], bytes[os2 + 69]]),
            i16::from_be_bytes([bytes[os2 + 72], bytes[os2 + 73]]),
        )
    } else {
        (hhea_asc, hhea_desc)
    };
    Some(f32::from(asc) - f32::from(desc))
}

fn bump_i16(bytes: &mut [u8], offset: usize, delta: i16) {
    if offset + 2 > bytes.len() {
        return;
    }
    let value = i16::from_be_bytes([bytes[offset], bytes[offset + 1]]);
    let value = value.saturating_add(delta);
    bytes[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
}

#[cfg(target_os = "windows")]
fn font_candidates() -> &'static [&'static str] {
    &[
        r"C:\Windows\Fonts\simhei.ttf",
        r"C:\Windows\Fonts\msyh.ttc",
        r"C:\Windows\Fonts\simsun.ttc",
    ]
}

#[cfg(target_os = "macos")]
fn font_candidates() -> &'static [&'static str] {
    &[
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/STHeiti Light.ttc",
    ]
}

#[cfg(target_os = "linux")]
fn font_candidates() -> &'static [&'static str] {
    &[
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
    ]
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn font_candidates() -> &'static [&'static str] {
    &[]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal valid sfnt: offset table + `hhea`/`OS/2` tables. hhea metrics
    /// are fixed at ascender 1536 / descender -512 (line height 2048); the
    /// OS/2 typo metrics and `USE_TYPO_METRICS` flag are configurable.
    fn fake_sfnt(
        hhea_line_gap: i16,
        typo_asc: i16,
        typo_desc: i16,
        typo_line_gap: i16,
        use_typo: bool,
    ) -> Vec<u8> {
        let mut hhea = vec![0u8; 36];
        hhea[4..6].copy_from_slice(&1536i16.to_be_bytes()); // ascender
        hhea[6..8].copy_from_slice(&(-512i16).to_be_bytes()); // descender
        hhea[8..10].copy_from_slice(&hhea_line_gap.to_be_bytes());

        let mut os2 = vec![0u8; 96];
        os2[62..64].copy_from_slice(&(if use_typo { 0x0080u16 } else { 0u16 }).to_be_bytes()); // fsSelection
        os2[68..70].copy_from_slice(&typo_asc.to_be_bytes()); // sTypoAscender
        os2[72..74].copy_from_slice(&typo_desc.to_be_bytes()); // sTypoDescender
        os2[74..76].copy_from_slice(&typo_line_gap.to_be_bytes());

        let tables = [(b"hhea".to_vec(), hhea), (b"OS/2".to_vec(), os2)];
        let mut bytes = vec![0u8; 12 + 16 * tables.len()];
        bytes[0..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        bytes[4..6].copy_from_slice(&(tables.len() as u16).to_be_bytes());
        let mut offset = bytes.len();
        for (i, (tag, table)) in tables.into_iter().enumerate() {
            let record = 12 + 16 * i;
            bytes[record..record + 4].copy_from_slice(&tag);
            bytes[record + 8..record + 12].copy_from_slice(&(offset as u32).to_be_bytes());
            bytes[record + 12..record + 16].copy_from_slice(&(table.len() as u32).to_be_bytes());
            bytes.extend_from_slice(&table);
            offset += table.len();
        }
        bytes
    }

    fn read_i16(bytes: &[u8], offset: usize) -> i16 {
        i16::from_be_bytes([bytes[offset], bytes[offset + 1]])
    }

    #[test]
    fn bumps_both_line_gap_metrics_by_the_requested_em_fraction() {
        let patched = patch_line_gap(fake_sfnt(0, 1536, -512, 0, false), 0.15);
        // Tables are appended in order: hhea(36) @44, OS/2(96) @80.
        // hhea line height = 1536 - (-512) = 2048; 0.15 * 2048 = 307.2 -> 307
        assert_eq!(read_i16(&patched, 44 + 8), 307); // hhea.lineGap
        assert_eq!(read_i16(&patched, 80 + 74), 307); // OS/2.sTypoLineGap
    }

    #[test]
    fn adds_to_an_existing_line_gap() {
        let patched = patch_line_gap(fake_sfnt(100, 1536, -512, 50, false), 0.1);
        // 0.1 * 2048 = 204.8 -> 205
        assert_eq!(read_i16(&patched, 44 + 8), 305);
        assert_eq!(read_i16(&patched, 80 + 74), 255);
    }

    #[test]
    fn uses_os2_typo_height_when_use_typo_metrics_is_set() {
        // USE_TYPO_METRICS switches the delta basis to the OS/2 typo metrics:
        // height = 1000 - (-100) = 1100; 0.15 * 1100 = 165, not the hhea 307.
        let patched = patch_line_gap(fake_sfnt(0, 1000, -100, 0, true), 0.15);
        assert_eq!(read_i16(&patched, 44 + 8), 165); // hhea.lineGap
        assert_eq!(read_i16(&patched, 80 + 74), 165); // OS/2.sTypoLineGap
    }

    #[test]
    fn leaves_non_sfnt_bytes_untouched() {
        assert_eq!(patch_line_gap(vec![0u8; 4], 0.15), vec![0u8; 4]);
        assert_eq!(patch_line_gap(Vec::<u8>::new(), 0.15), Vec::<u8>::new());
    }

    #[test]
    fn zero_fraction_is_a_no_op() {
        let original = fake_sfnt(0, 1536, -512, 0, false);
        assert_eq!(patch_line_gap(original.clone(), 0.0), original);
    }

    #[test]
    fn real_egui_fonts_grow_row_height() {
        use ab_glyph::{Font as _, ScaleFont as _};

        for font in [
            epaint_default_fonts::UBUNTU_LIGHT,
            epaint_default_fonts::HACK_REGULAR,
        ] {
            let original = ab_glyph::FontVec::try_from_vec(font.to_vec()).unwrap();
            let patched = ab_glyph::FontVec::try_from_vec(patch_line_gap(
                font.to_vec(),
                EXTRA_LINE_GAP_EM,
            ))
            .unwrap();
            let expected = EXTRA_LINE_GAP_EM * 13.0;
            let actual =
                patched.as_scaled(13.0).line_gap() - original.as_scaled(13.0).line_gap();
            assert!(
                (actual - expected).abs() < 0.1,
                "line gap grew by {actual}pt, expected {expected}pt"
            );
        }
    }
}
