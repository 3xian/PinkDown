#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod app;
mod dialog;
mod document;
mod export;
mod preview;
mod settings;
mod theme;
mod update;
mod window;

use std::path::PathBuf;

use app::PinkDown;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let initial_path = std::env::args_os().nth(1).map(PathBuf::from);
    let viewport = egui::ViewportBuilder::default()
        .with_title("PinkDown")
        .with_decorations(true)
        .with_transparent(false)
        .with_has_shadow(true)
        .with_resizable(true)
        .with_inner_size([1280.0, 820.0])
        .with_min_inner_size([760.0, 540.0])
        .with_icon(theme::icon_data());

    // macOS: hide the native title chrome while keeping traffic lights.
    // Content draws edge-to-edge; the app toolbar is the drag region.
    #[cfg(target_os = "macos")]
    let viewport = viewport
        .with_fullsize_content_view(true)
        .with_titlebar_shown(false)
        .with_title_shown(false);

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "PinkDown",
        options,
        Box::new(move |cc| Ok(Box::new(PinkDown::new(cc, initial_path)))),
    )
}
