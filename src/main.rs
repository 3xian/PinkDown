#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod app;
mod dialog;
mod document;
mod preview;
mod theme;
mod update;
mod window;

use std::path::PathBuf;

use app::PinkDown;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let initial_path = std::env::args_os().nth(1).map(PathBuf::from);
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
        Box::new(move |cc| Ok(Box::new(PinkDown::new(cc, initial_path)))),
    )
}
