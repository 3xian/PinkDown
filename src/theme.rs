use std::fs;

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

pub fn icon_data() -> egui::IconData {
    let mut rgba = Vec::with_capacity(64 * 64 * 4);
    for y in 0..64_u32 {
        for x in 0..64_u32 {
            let inside = x > 8 && x < 56 && y > 6 && y < 58;
            let fold = x > 42 && y < 22;
            let color = if inside && !fold {
                [235, 111, 146, 255]
            } else if fold {
                [246, 193, 119, 255]
            } else {
                [25, 23, 36, 255]
            };
            rgba.extend_from_slice(&color);
        }
    }
    egui::IconData {
        rgba,
        width: 64,
        height: 64,
    }
}

pub fn configure(ctx: &egui::Context) {
    configure_fonts(ctx);

    let mut style = (*ctx.style()).clone();
    style.visuals = egui::Visuals::dark();
    style.visuals.panel_fill = Color32::TRANSPARENT;
    style.visuals.window_fill = SURFACE;
    style.visuals.extreme_bg_color = BASE;
    style.visuals.faint_bg_color = OVERLAY;
    style.visuals.widgets.noninteractive.bg_fill = SURFACE;
    style.visuals.widgets.inactive.bg_fill = OVERLAY;
    style.visuals.widgets.inactive.weak_bg_fill = OVERLAY;
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, HIGHLIGHT_MED);
    style.visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(8);
    style.visuals.widgets.hovered.bg_fill = HIGHLIGHT_MED;
    style.visuals.widgets.hovered.weak_bg_fill = HIGHLIGHT_MED;
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, HIGHLIGHT_HIGH);
    style.visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(8);
    style.visuals.widgets.active.bg_fill = HIGHLIGHT_HIGH;
    style.visuals.widgets.active.weak_bg_fill = HIGHLIGHT_HIGH;
    style.visuals.widgets.active.corner_radius = egui::CornerRadius::same(8);
    style.visuals.selection.bg_fill = PINE;
    style.visuals.selection.stroke.color = FOAM;
    style.visuals.hyperlink_color = FOAM;
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
        FontId::new(13.0, FontFamily::Monospace),
    );
    style.url_in_tooltip = true;
    ctx.set_style(style);
}

fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let mut configured = false;

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
        configured = true;
    }

    if configured {
        ctx.set_fonts(fonts);
    }
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
