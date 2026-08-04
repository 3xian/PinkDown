#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod app;
mod document;
mod theme;
mod update;
mod window;

use app::PinkDown;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("PinkDown")
            .with_decorations(true)
            .with_transparent(false)
            .with_has_shadow(true)
            .with_resizable(true)
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([760.0, 540.0])
            .with_icon(theme::icon_data()),
        ..Default::default()
    };

    eframe::run_native(
        "PinkDown",
        options,
        Box::new(|cc| Ok(Box::new(PinkDown::new(cc)))),
    )
}
