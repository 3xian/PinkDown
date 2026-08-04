#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    process::{self, Command},
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
};

use chardetng::EncodingDetector;
use eframe::{
    egui,
    egui::{Color32, FontData, FontDefinitions, FontFamily, FontId, RichText, TextFormat},
};
use encoding_rs::{UTF_16BE, UTF_16LE};
use rfd::FileDialog;
use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};

// Rosé Pine — official default palette: https://rosepinetheme.com/palette/
const BASE: Color32 = Color32::from_rgb(25, 23, 36);
const SURFACE: Color32 = Color32::from_rgb(31, 29, 46);
const OVERLAY: Color32 = Color32::from_rgb(38, 35, 58);
const MUTED: Color32 = Color32::from_rgb(110, 106, 134);
const SUBTLE: Color32 = Color32::from_rgb(144, 140, 170);
const TEXT: Color32 = Color32::from_rgb(224, 222, 244);
const LOVE: Color32 = Color32::from_rgb(235, 111, 146);
const GOLD: Color32 = Color32::from_rgb(246, 193, 119);
const ROSE: Color32 = Color32::from_rgb(235, 188, 186);
const PINE: Color32 = Color32::from_rgb(49, 116, 143);
const FOAM: Color32 = Color32::from_rgb(156, 207, 216);
const IRIS: Color32 = Color32::from_rgb(196, 167, 231);
const HIGHLIGHT_LOW: Color32 = Color32::from_rgb(33, 32, 46);
const HIGHLIGHT_MED: Color32 = Color32::from_rgb(64, 61, 82);
const HIGHLIGHT_HIGH: Color32 = Color32::from_rgb(82, 79, 103);
const GITHUB_TAGS_URL: &str = "https://api.github.com/repos/3xian/PinkDown/tags?per_page=100";
const GITHUB_RELEASES_URL: &str = "https://github.com/3xian/PinkDown/releases/download";
const WINDOWS_RELEASE_ASSET: &str = "pinkdown-windows-x64.exe";

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("PinkDown")
            // Start with a native frame. On Windows we remove only WS_CAPTION
            // after creation, retaining DWM corners, shadow, and resize borders.
            .with_decorations(true)
            .with_transparent(false)
            .with_has_shadow(true)
            .with_resizable(true)
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([760.0, 540.0])
            .with_icon(icon_data()),
        ..Default::default()
    };
    eframe::run_native(
        "PinkDown",
        options,
        Box::new(|cc| Ok(Box::new(PinkDown::new(cc)))),
    )
}

fn icon_data() -> egui::IconData {
    let mut rgba = Vec::with_capacity(64 * 64 * 4);
    for y in 0..64u32 {
        for x in 0..64u32 {
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

struct PinkDown {
    source: String,
    saved_source: String,
    file_path: Option<PathBuf>,
    status: String,
    show_help: bool,
    update_receiver: Option<Receiver<UpdateResult>>,
    #[cfg(target_os = "windows")]
    native_frame_passes: u8,
}

impl PinkDown {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_cjk_font(&cc.egui_ctx);
        configure_style(&cc.egui_ctx);
        Self {
            source: "# Welcome to PinkDown\n\nA calm place for your **ideas**, notes, and writing.\n\n> Write on the left. See it take shape on the right.\n\n## A tiny, beautiful editor\n\n- Open any `.md` file\n- Save as you work\n- Stay focused\n\n```rust\nfn hello() {\n    println!(\"Hello, PinkDown!\");\n}\n```\n\n---\n\nMade with warmth and precision.".into(),
            saved_source: String::new(),
            file_path: None,
            status: "Ready to write".into(),
            show_help: false,
            update_receiver: None,
            #[cfg(target_os = "windows")]
            native_frame_passes: 0,
        }
    }

    fn is_dirty(&self) -> bool {
        self.source != self.saved_source && self.file_path.is_some()
    }

    fn open(&mut self) {
        if let Some(path) = FileDialog::new()
            .add_filter("Markdown", &["md", "markdown", "mdx", "txt"])
            .pick_file()
        {
            self.open_path(path);
        }
    }

    fn open_path(&mut self, path: PathBuf) {
        match read_markdown_file(&path) {
            Ok((text, encoding)) => {
                self.source = text;
                self.saved_source = self.source.clone();
                self.status = format!("Opened {} · {encoding}", display_name(&path));
                self.file_path = Some(path);
            }
            Err(error) => self.status = format!("Could not open file: {error}"),
        }
    }

    fn save(&mut self, force_dialog: bool) {
        let path = if force_dialog || self.file_path.is_none() {
            FileDialog::new()
                .add_filter("Markdown", &["md"])
                .set_file_name("untitled.md")
                .save_file()
        } else {
            self.file_path.clone()
        };
        if let Some(path) = path {
            match fs::write(&path, &self.source) {
                Ok(()) => {
                    self.saved_source = self.source.clone();
                    self.status = format!("Saved {}", display_name(&path));
                    self.file_path = Some(path);
                }
                Err(error) => self.status = format!("Could not save file: {error}"),
            }
        }
    }

    fn check_for_updates(&mut self) {
        if self.update_receiver.is_some() {
            return;
        }

        let (sender, receiver) = mpsc::channel();
        self.update_receiver = Some(receiver);
        self.status = "Checking for updates…".into();
        thread::spawn(move || {
            let _ = sender.send(check_and_install_update());
        });
    }

    fn poll_update(&mut self, ctx: &egui::Context) {
        let Some(receiver) = &self.update_receiver else {
            return;
        };
        match receiver.try_recv() {
            Ok(UpdateResult::Status(message)) => {
                self.status = message;
                self.update_receiver = None;
            }
            Ok(UpdateResult::Restarting(message)) => {
                self.status = message;
                self.update_receiver = None;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Err(TryRecvError::Empty) => {
                ctx.request_repaint_after(std::time::Duration::from_millis(100))
            }
            Err(TryRecvError::Disconnected) => {
                self.status = "Update check did not complete".into();
                self.update_receiver = None;
            }
        }
    }
}

#[derive(Deserialize)]
struct GitHubTag {
    name: String,
}

enum UpdateResult {
    Status(String),
    Restarting(String),
}

fn check_and_install_update() -> UpdateResult {
    let result = (|| {
        let latest_tag = latest_github_tag()?;
        let current_version = Version::parse(env!("CARGO_PKG_VERSION"))
            .map_err(|error| format!("Invalid current version: {error}"))?;
        let latest_version = version_from_tag(&latest_tag)?;

        if latest_version <= current_version {
            return Ok(UpdateResult::Status(format!(
                "PinkDown v{current_version} is up to date"
            )));
        }

        #[cfg(target_os = "windows")]
        {
            let downloaded_update = download_windows_update(&latest_tag)?;
            schedule_windows_update(&downloaded_update)?;
            Ok(UpdateResult::Restarting(format!(
                "Installing PinkDown {latest_tag}; the app will restart"
            )))
        }

        #[cfg(not(target_os = "windows"))]
        {
            Err("Automatic updates are currently available on Windows only".into())
        }
    })();

    result.unwrap_or_else(UpdateResult::Status)
}

fn latest_github_tag() -> Result<String, String> {
    let tags: Vec<GitHubTag> = ureq::get(GITHUB_TAGS_URL)
        .set(
            "User-Agent",
            concat!("PinkDown/", env!("CARGO_PKG_VERSION")),
        )
        .call()
        .map_err(|error| format!("Could not contact GitHub: {error}"))?
        .into_json()
        .map_err(|error| format!("Could not read GitHub tags: {error}"))?;

    tags.into_iter()
        .filter_map(|tag| {
            version_from_tag(&tag.name)
                .ok()
                .map(|version| (version, tag.name))
        })
        .max_by(|left, right| left.0.cmp(&right.0))
        .map(|(_, tag)| tag)
        .ok_or_else(|| "No semantic-version tags found on GitHub".into())
}

fn version_from_tag(tag: &str) -> Result<Version, String> {
    Version::parse(tag.trim_start_matches('v'))
        .map_err(|error| format!("GitHub tag {tag:?} is not a semantic version: {error}"))
}

#[cfg(target_os = "windows")]
fn download_windows_update(tag: &str) -> Result<PathBuf, String> {
    let asset_url = format!("{GITHUB_RELEASES_URL}/{tag}/{WINDOWS_RELEASE_ASSET}");
    let checksum_url = format!("{asset_url}.sha256");
    let expected_checksum = download_text(&checksum_url)?
        .split_whitespace()
        .next()
        .filter(|checksum| checksum.len() == 64 && checksum.chars().all(|c| c.is_ascii_hexdigit()))
        .ok_or_else(|| "Release checksum is missing or invalid".to_owned())?
        .to_ascii_lowercase();
    let destination = std::env::temp_dir().join(format!("pinkdown-{tag}-{}.exe", process::id()));

    let result = (|| {
        let response = ureq::get(&asset_url)
            .set(
                "User-Agent",
                concat!("PinkDown/", env!("CARGO_PKG_VERSION")),
            )
            .call()
            .map_err(|error| format!("Could not download {tag}: {error}"))?;
        let mut source = response.into_reader();
        let mut file = fs::File::create(&destination)
            .map_err(|error| format!("Could not create update file: {error}"))?;
        io::copy(&mut source, &mut file)
            .map_err(|error| format!("Could not save update: {error}"))?;

        let actual_checksum = sha256_file(&destination)?;
        if actual_checksum != expected_checksum {
            return Err("Downloaded update did not match its release checksum".into());
        }
        Ok(destination.clone())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&destination);
    }
    result
}

#[cfg(target_os = "windows")]
fn download_text(url: &str) -> Result<String, String> {
    let mut response = ureq::get(url)
        .set(
            "User-Agent",
            concat!("PinkDown/", env!("CARGO_PKG_VERSION")),
        )
        .call()
        .map_err(|error| format!("Could not download release checksum: {error}"))?
        .into_reader();
    let mut text = String::new();
    response
        .read_to_string(&mut text)
        .map_err(|error| format!("Could not read release checksum: {error}"))?;
    Ok(text)
}

#[cfg(target_os = "windows")]
fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file =
        fs::File::open(path).map_err(|error| format!("Could not verify update: {error}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 32 * 1024];
    loop {
        let bytes_read = file
            .read(&mut buffer)
            .map_err(|error| format!("Could not verify update: {error}"))?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(target_os = "windows")]
fn schedule_windows_update(downloaded_update: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let current_exe = std::env::current_exe()
        .map_err(|error| format!("Could not locate the running app: {error}"))?;
    let script_path = std::env::temp_dir().join(format!("pinkdown-update-{}.ps1", process::id()));
    let parent_process = process::id();
    let quote = |path: &Path| path.display().to_string().replace('\'', "''");
    let script = format!(
        "$ErrorActionPreference = 'Stop'\n$parent = Get-Process -Id {parent_process} -ErrorAction SilentlyContinue\nif ($parent) {{ Wait-Process -Id {parent_process} }}\nMove-Item -LiteralPath '{}' -Destination '{}' -Force\nStart-Process -FilePath '{}'\nRemove-Item -LiteralPath $PSCommandPath -Force\n",
        quote(downloaded_update),
        quote(&current_exe),
        quote(&current_exe),
    );
    fs::write(&script_path, script)
        .map_err(|error| format!("Could not prepare updater: {error}"))?;
    Command::new("powershell.exe")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(&script_path)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|error| format!("Could not start updater: {error}"))?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn configure_native_window(window: &impl raw_window_handle::HasWindowHandle) {
    use raw_window_handle::RawWindowHandle;
    use windows_sys::Win32::{
        Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND},
        UI::WindowsAndMessaging::{
            GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_STYLE, SWP_FRAMECHANGED,
            SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WS_CAPTION, WS_MAXIMIZEBOX,
            WS_MINIMIZEBOX, WS_SYSMENU, WS_THICKFRAME,
        },
    };

    let Ok(handle) = window.window_handle() else {
        return;
    };
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return;
    };
    let hwnd = handle.hwnd.get() as *mut core::ffi::c_void;

    // SAFETY: `hwnd` comes from eframe's live CreationContext and all calls are
    // made on the UI thread during window creation.
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        let borderless_native_style =
            (style & !WS_CAPTION) | WS_THICKFRAME | WS_MINIMIZEBOX | WS_MAXIMIZEBOX | WS_SYSMENU;
        SetWindowLongPtrW(hwnd, GWL_STYLE, borderless_native_style as isize);

        let corner_preference = DWMWCP_ROUND;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE as u32,
            (&corner_preference as *const _) as *const core::ffi::c_void,
            std::mem::size_of_val(&corner_preference) as u32,
        );

        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            0,
            0,
            0,
            0,
            SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
}

impl eframe::App for PinkDown {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        #[cfg(target_os = "windows")]
        if self.native_frame_passes < 4 {
            configure_native_window(frame);
            self.native_frame_passes += 1;
            ctx.request_repaint_after(std::time::Duration::from_millis(80));
        }
        self.poll_update(ctx);

        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::O)) {
            self.open();
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::S)) {
            self.save(false);
        }
        if let Some(path) = ctx
            .input(|i| i.raw.dropped_files.clone())
            .into_iter()
            .find_map(|file| file.path)
        {
            self.open_path(path);
        }

        paint_window_shell(ctx);

        egui::TopBottomPanel::top("window-toolbar")
            .exact_height(64.0)
            .show_separator_line(false)
            .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(20, 10)))
            .show(ctx, |ui| {
                title_bar(ui, ctx, self);
            });

        egui::TopBottomPanel::bottom("statusbar")
            .exact_height(36.0)
            .show_separator_line(false)
            .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(8, 4)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(14.0);
                    let title = self
                        .file_path
                        .as_ref()
                        .map_or_else(|| "Untitled".to_owned(), |p| display_name(p));
                    ui.label(RichText::new(title).size(12.0).color(if self.is_dirty() {
                        GOLD
                    } else {
                        SUBTLE
                    }));
                    ui.label(RichText::new(&self.status).size(11.0).color(MUTED));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(14.0);
                        ui.label(
                            RichText::new(format!(
                                "{} words",
                                self.source.split_whitespace().count()
                            ))
                            .size(11.0)
                            .color(MUTED),
                        );
                        ui.label(
                            RichText::new(format!("{} lines", self.source.lines().count()))
                                .size(11.0)
                                .color(MUTED),
                        );
                    });
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(20, 8)))
            .show(ctx, |ui| {
                let available = ui.available_width();
                ui.columns(2, |columns| {
                    columns[0].set_width((available - 12.0) * 0.5);
                    source_panel(&mut columns[0], &mut self.source);
                    preview_panel(&mut columns[1], &self.source);
                });
            });

        if self.show_help {
            help_window(ctx, &mut self.show_help);
        }
    }
}

fn configure_cjk_font(ctx: &egui::Context) {
    #[cfg(target_os = "windows")]
    let candidates = [
        r"C:\Windows\Fonts\simhei.ttf",
        r"C:\Windows\Fonts\msyh.ttc",
        r"C:\Windows\Fonts\simsun.ttc",
    ];
    #[cfg(target_os = "macos")]
    let candidates = [
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/STHeiti Light.ttc",
    ];
    #[cfg(target_os = "linux")]
    let candidates = [
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
    ];

    #[cfg(target_os = "windows")]
    let brand_candidates = [
        r"C:\Windows\Fonts\simhei.ttf",
        r"C:\Windows\Fonts\arialbd.ttf",
    ];
    #[cfg(target_os = "macos")]
    let brand_candidates = [
        "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
        "/System/Library/Fonts/Supplemental/Verdana Bold.ttf",
    ];
    #[cfg(target_os = "linux")]
    let brand_candidates = [
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/truetype/liberation2/LiberationSans-Bold.ttf",
    ];

    let mut fonts = FontDefinitions::default();
    let mut configured = false;
    if let Some(bytes) = candidates.into_iter().find_map(|path| fs::read(path).ok()) {
        let font_name = "system-cjk".to_owned();
        fonts.font_data.insert(
            font_name.clone(),
            std::sync::Arc::new(FontData::from_owned(bytes)),
        );
        fonts
            .families
            .get_mut(&FontFamily::Proportional)
            .expect("default proportional font family")
            .push(font_name.clone());
        fonts
            .families
            .get_mut(&FontFamily::Monospace)
            .expect("default monospace font family")
            .push(font_name);
        configured = true;
    }
    if let Some(bytes) = brand_candidates
        .into_iter()
        .find_map(|path| fs::read(path).ok())
    {
        let font_name = "brand-bold".to_owned();
        fonts.font_data.insert(
            font_name.clone(),
            std::sync::Arc::new(FontData::from_owned(bytes)),
        );
        fonts
            .families
            .insert(FontFamily::Name(font_name.clone().into()), vec![font_name]);
        configured = true;
    }
    if configured {
        ctx.set_fonts(fonts);
    }
}

fn configure_style(ctx: &egui::Context) {
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
    ctx.set_style(style);
}

fn paint_window_shell(ctx: &egui::Context) {
    // The native window owns the resize border, corners, and DWM shadow. We only
    // paint the client area because the system title bar is intentionally hidden.
    let shell = ctx.screen_rect();
    let painter = ctx.layer_painter(egui::LayerId::background());
    painter.rect_filled(shell, 0.0, BASE);
}

#[derive(Clone, Copy)]
enum WindowButton {
    Minimize,
    Maximize,
    Restore,
    Close,
}

fn title_bar(ui: &mut egui::Ui, ctx: &egui::Context, app: &mut PinkDown) {
    let drag = ui.interact(
        ui.max_rect(),
        ui.id().with("title-drag"),
        egui::Sense::drag(),
    );
    if drag.drag_started() {
        ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
    if drag.double_clicked() {
        let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
    }

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.add_space(1.0);
            gradient_label(ui, "PinkDown", 14.0);
            ui.label(RichText::new("MARKDOWN STUDIO").size(8.0).color(MUTED));
        });

        ui.add_space(16.0);
        if toolbar_button(ui, "Open").clicked() {
            app.open();
        }
        if toolbar_button(ui, "Save").clicked() {
            app.save(false);
        }
        if toolbar_button(ui, "Save as").clicked() {
            app.save(true);
        }
        if toolbar_button(ui, "Check updates")
            .on_hover_text("Download and install the latest GitHub release")
            .clicked()
        {
            app.check_for_updates();
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if window_button(ui, WindowButton::Close, "Close").clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
            let kind = if maximized {
                WindowButton::Restore
            } else {
                WindowButton::Maximize
            };
            if window_button(ui, kind, if maximized { "Restore" } else { "Maximize" }).clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
            }
            if window_button(ui, WindowButton::Minimize, "Minimize").clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            }
            ui.add_space(8.0);
            if ui
                .add_sized(
                    [32.0, 30.0],
                    egui::Button::new(RichText::new("?").size(14.0).color(SUBTLE)).frame(false),
                )
                .on_hover_text("Markdown guide")
                .clicked()
            {
                app.show_help = !app.show_help;
            }
        });
    });
}

fn toolbar_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let size = egui::vec2(
        match label {
            "Save as" => 64.0,
            "Check updates" => 96.0,
            _ => 52.0,
        },
        30.0,
    );
    let hover =
        ui.rect_contains_pointer(egui::Rect::from_min_size(ui.next_widget_position(), size));
    let text = if hover {
        RichText::new(label)
            .size(12.0)
            .color(TEXT)
            .family(FontFamily::Name("brand-bold".into()))
    } else {
        RichText::new(label).size(12.0).color(SUBTLE)
    };
    ui.add_sized(size, egui::Button::new(text).frame(false))
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn gradient_label(ui: &mut egui::Ui, text: &str, size: f32) {
    let mut job = egui::text::LayoutJob::default();
    let characters: Vec<char> = text.chars().collect();
    let denominator = characters.len().saturating_sub(1).max(1) as f32;
    for (index, character) in characters.into_iter().enumerate() {
        let progress = index as f32 / denominator;
        let color = if progress <= 0.5 {
            lerp_color(ROSE, IRIS, progress * 2.0)
        } else {
            lerp_color(IRIS, FOAM, (progress - 0.5) * 2.0)
        };
        job.append(
            &character.to_string(),
            0.0,
            TextFormat {
                font_id: FontId::new(size, FontFamily::Name("brand-bold".into())),
                color,
                ..Default::default()
            },
        );
    }
    ui.label(job);
}

fn lerp_color(from: Color32, to: Color32, amount: f32) -> Color32 {
    let mix = |start: u8, end: u8| {
        (start as f32 + (end as f32 - start as f32) * amount.clamp(0.0, 1.0)).round() as u8
    };
    Color32::from_rgb(
        mix(from.r(), to.r()),
        mix(from.g(), to.g()),
        mix(from.b(), to.b()),
    )
}

fn window_button(ui: &mut egui::Ui, kind: WindowButton, tooltip: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(36.0, 30.0), egui::Sense::click());
    let close = matches!(kind, WindowButton::Close);
    if response.hovered() || response.is_pointer_button_down_on() {
        ui.painter().rect_filled(
            rect,
            egui::CornerRadius::same(8),
            if close { LOVE } else { HIGHLIGHT_LOW },
        );
    }
    let color = if close && response.hovered() {
        TEXT
    } else {
        SUBTLE
    };
    let stroke = egui::Stroke::new(1.3, color);
    let center = rect.center();
    match kind {
        WindowButton::Minimize => ui.painter().line_segment(
            [
                center + egui::vec2(-5.0, 3.0),
                center + egui::vec2(5.0, 3.0),
            ],
            stroke,
        ),
        WindowButton::Maximize => ui.painter().rect_stroke(
            egui::Rect::from_center_size(center, egui::vec2(9.0, 8.0)),
            1.0,
            stroke,
            egui::StrokeKind::Inside,
        ),
        WindowButton::Restore => {
            ui.painter().rect_stroke(
                egui::Rect::from_min_size(center + egui::vec2(-3.0, -5.0), egui::vec2(8.0, 7.0)),
                1.0,
                stroke,
                egui::StrokeKind::Inside,
            );
            ui.painter().rect_stroke(
                egui::Rect::from_min_size(center + egui::vec2(-5.0, -2.0), egui::vec2(8.0, 7.0)),
                1.0,
                stroke,
                egui::StrokeKind::Inside,
            )
        }
        WindowButton::Close => {
            ui.painter().line_segment(
                [
                    center + egui::vec2(-4.0, -4.0),
                    center + egui::vec2(4.0, 4.0),
                ],
                stroke,
            );
            ui.painter().line_segment(
                [
                    center + egui::vec2(4.0, -4.0),
                    center + egui::vec2(-4.0, 4.0),
                ],
                stroke,
            )
        }
    };
    response.on_hover_text(tooltip)
}

fn source_panel(ui: &mut egui::Ui, source: &mut String) {
    egui::Frame::new()
        .fill(SURFACE)
        .stroke(egui::Stroke::new(1.0, HIGHLIGHT_LOW))
        .corner_radius(egui::CornerRadius::same(12))
        .inner_margin(egui::Margin::symmetric(18, 16))
        .show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            ui.horizontal(|ui| {
                ui.label(RichText::new("SOURCE").size(11.0).strong().color(MUTED));
                ui.label(RichText::new("MARKDOWN").size(10.0).color(MUTED));
            });
            ui.add_space(8.0);
            egui::Frame::NONE.fill(SURFACE).show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("source-scroll")
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        let width = ui.available_width();
                        ui.add_sized(
                            [width, ui.available_height().max(200.0)],
                            egui::TextEdit::multiline(source)
                                .font(egui::TextStyle::Monospace)
                                .text_color(TEXT)
                                .frame(false)
                                .code_editor()
                                .desired_rows(30)
                                .lock_focus(true),
                        );
                    });
            });
        });
}

fn preview_panel(ui: &mut egui::Ui, source: &str) {
    egui::Frame::new()
        .fill(BASE)
        .stroke(egui::Stroke::new(1.0, HIGHLIGHT_LOW))
        .corner_radius(egui::CornerRadius::same(12))
        .inner_margin(egui::Margin::symmetric(20, 16))
        .show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            ui.horizontal(|ui| {
                ui.label(RichText::new("PREVIEW").size(11.0).strong().color(MUTED));
                ui.label(RichText::new("LIVE RENDER").size(10.0).color(MUTED));
            });
            ui.add_space(8.0);
            egui::Frame::NONE
                .fill(Color32::TRANSPARENT)
                .inner_margin(egui::Margin::symmetric(14, 8))
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("preview-scroll")
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            render_markdown(ui, source);
                            ui.add_space(30.0);
                        });
                });
        });
}

fn render_markdown(ui: &mut egui::Ui, source: &str) {
    let lines: Vec<&str> = source.lines().collect();
    let mut index = 0;
    let mut in_code = false;
    let mut code = String::new();
    while index < lines.len() {
        let line = lines[index];
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            if in_code {
                render_code_block(ui, &code);
                ui.add_space(9.0);
                code.clear();
            }
            in_code = !in_code;
        } else if in_code {
            code.push_str(line);
            code.push('\n');
        } else if let Some((headers, rows, next_index)) = parse_table(&lines, index) {
            render_table(ui, &headers, &rows);
            ui.add_space(10.0);
            index = next_index;
            continue;
        } else if trimmed.is_empty() {
            ui.add_space(7.0);
        } else if is_rule(trimmed) {
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);
        } else if let Some((level, text)) = heading(trimmed) {
            let size = match level {
                1 => 31.0,
                2 => 24.0,
                3 => 20.0,
                _ => 17.0,
            };
            ui.add_space(if level == 1 { 5.0 } else { 3.0 });
            inline_label(ui, text, size, true, if level == 1 { ROSE } else { TEXT });
            ui.add_space(4.0);
        } else if let Some(quote) = trimmed.strip_prefix("> ") {
            egui::Frame::new()
                .fill(OVERLAY)
                .inner_margin(egui::Margin::symmetric(12, 9))
                .show(ui, |ui| {
                    inline_label(ui, quote, 15.0, false, SUBTLE);
                });
            ui.add_space(6.0);
        } else if let Some(item) = list_item(trimmed) {
            ui.horizontal_wrapped(|ui| {
                let (mark, text) = if let Some(rest) = item.strip_prefix("[ ] ") {
                    ("○", rest)
                } else if let Some(rest) = item
                    .strip_prefix("[x] ")
                    .or_else(|| item.strip_prefix("[X] "))
                {
                    ("●", rest)
                } else {
                    ("•", item)
                };
                ui.label(RichText::new(mark).size(16.0).color(if mark == "●" {
                    FOAM
                } else {
                    IRIS
                }));
                inline_label(ui, text, 15.0, false, TEXT);
            });
        } else {
            inline_label(ui, trimmed, 15.0, false, TEXT);
        }
        index += 1;
    }
    if in_code && !code.is_empty() {
        render_code_block(ui, &code);
    }
}

fn render_code_block(ui: &mut egui::Ui, code: &str) {
    egui::Frame::new()
        .fill(OVERLAY)
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(egui::Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.add(
                egui::Label::new(
                    RichText::new(code)
                        .font(FontId::new(13.0, FontFamily::Monospace))
                        .color(FOAM),
                )
                .selectable(true)
                .wrap_mode(egui::TextWrapMode::Extend),
            );
        });
}

fn parse_table<'a>(
    lines: &[&'a str],
    start: usize,
) -> Option<(Vec<&'a str>, Vec<Vec<&'a str>>, usize)> {
    let headers = table_cells(*lines.get(start)?)?;
    let separator = table_cells(*lines.get(start + 1)?)?;
    if separator.len() != headers.len() || !separator.iter().all(|cell| is_table_rule(cell)) {
        return None;
    }

    let mut rows = Vec::new();
    let mut next = start + 2;
    while let Some(line) = lines.get(next) {
        let Some(mut cells) = table_cells(line) else {
            break;
        };
        cells.resize(headers.len(), "");
        cells.truncate(headers.len());
        rows.push(cells);
        next += 1;
    }
    Some((headers, rows, next))
}

fn table_cells(line: &str) -> Option<Vec<&str>> {
    let trimmed = line.trim();
    if !trimmed.contains('|') {
        return None;
    }
    let content = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let content = content.strip_suffix('|').unwrap_or(content);
    let cells: Vec<_> = content.split('|').map(str::trim).collect();
    (cells.len() >= 2).then_some(cells)
}

fn is_table_rule(cell: &str) -> bool {
    let rule = cell.trim().trim_matches(':');
    rule.len() >= 3 && rule.chars().all(|character| character == '-')
}

fn render_table(ui: &mut egui::Ui, headers: &[&str], rows: &[Vec<&str>]) {
    let table_width = ui.available_width();
    egui::Frame::new()
        .fill(OVERLAY)
        .corner_radius(egui::CornerRadius::same(9))
        .inner_margin(1.0)
        .show(ui, |ui| {
            ui.set_width((table_width - 2.0).max(120.0));
            table_visual_row(ui, headers, true, false);
            for (row_index, row) in rows.iter().enumerate() {
                table_visual_row(ui, row, false, row_index % 2 == 1);
            }
        });
}

fn table_visual_row(ui: &mut egui::Ui, cells: &[&str], header: bool, alternate: bool) {
    let fill = if header {
        HIGHLIGHT_MED
    } else if alternate {
        HIGHLIGHT_LOW
    } else {
        SURFACE
    };
    egui::Frame::new()
        .fill(fill)
        .corner_radius(if header {
            egui::CornerRadius::same(8)
        } else {
            egui::CornerRadius::ZERO
        })
        .inner_margin(egui::Margin::symmetric(12, 9))
        .show(ui, |ui| {
            let columns = cells.len().max(1);
            let gap = 14.0;
            let cell_width = ((ui.available_width() - gap * (columns.saturating_sub(1)) as f32)
                / columns as f32)
                .max(40.0);
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for cell in cells {
                    ui.allocate_ui_with_layout(
                        egui::vec2(cell_width, 0.0),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            if header {
                                ui.label(RichText::new(*cell).size(13.0).strong().color(TEXT));
                            } else {
                                inline_label(ui, cell, 13.0, false, SUBTLE);
                            }
                        },
                    );
                }
            });
        });
}

fn heading(line: &str) -> Option<(usize, &str)> {
    let count = line.bytes().take_while(|b| *b == b'#').count();
    (count > 0 && count <= 6 && line.as_bytes().get(count) == Some(&b' '))
        .then(|| (count, line[count + 1..].trim()))
}

fn is_rule(line: &str) -> bool {
    matches!(line, "---" | "***" | "___")
}

fn list_item(line: &str) -> Option<&str> {
    line.strip_prefix("- ")
        .or_else(|| line.strip_prefix("* "))
        .or_else(|| {
            let dot = line.find(". ")?;
            (line[..dot].chars().all(|c| c.is_ascii_digit())).then(|| &line[dot + 2..])
        })
}

fn inline_label(ui: &mut egui::Ui, text: &str, size: f32, strong: bool, color: Color32) {
    let mut job = egui::text::LayoutJob::default();
    let mut rest = text;
    while !rest.is_empty() {
        let marker = ["**", "`", "["]
            .iter()
            .filter_map(|m| rest.find(m).map(|at| (at, *m)))
            .min_by_key(|(at, _)| *at);
        let Some((at, marker)) = marker else {
            append(&mut job, rest, size, color, strong, false);
            break;
        };
        if at > 0 {
            append(&mut job, &rest[..at], size, color, strong, false);
            rest = &rest[at..];
            continue;
        }
        if marker == "**" {
            if let Some(end) = rest[2..].find("**") {
                append(&mut job, &rest[2..end + 2], size, color, true, false);
                rest = &rest[end + 4..];
            } else {
                append(&mut job, "**", size, color, strong, false);
                rest = &rest[2..];
            }
        } else if marker == "`" {
            if let Some(end) = rest[1..].find('`') {
                append(&mut job, &rest[1..end + 1], size - 1.0, FOAM, false, true);
                rest = &rest[end + 2..];
            } else {
                append(&mut job, "`", size, color, strong, false);
                rest = &rest[1..];
            }
        } else if let Some(close) = rest.find("](") {
            if let Some(end) = rest[close + 2..].find(')') {
                append(&mut job, &rest[1..close], size, FOAM, false, false);
                rest = &rest[close + end + 3..];
            } else {
                append(&mut job, "[", size, color, strong, false);
                rest = &rest[1..];
            }
        } else {
            append(&mut job, "[", size, color, strong, false);
            rest = &rest[1..];
        }
    }
    job.wrap.max_width = ui.available_width();
    ui.label(job);
}

fn append(
    job: &mut egui::text::LayoutJob,
    text: &str,
    size: f32,
    color: Color32,
    strong: bool,
    code: bool,
) {
    job.append(
        text,
        0.0,
        TextFormat {
            font_id: FontId::new(
                size,
                if code {
                    FontFamily::Monospace
                } else {
                    FontFamily::Proportional
                },
            ),
            color,
            italics: false,
            underline: egui::Stroke::NONE,
            strikethrough: egui::Stroke::NONE,
            valign: egui::Align::Center,
            background: if code { OVERLAY } else { Color32::TRANSPARENT },
            ..Default::default()
        },
    );
    if strong { /* weight is applied by the surrounding heading style; egui's layout format has no weight field. */
    }
}

fn help_window(ctx: &egui::Context, open: &mut bool) {
    egui::Window::new("Markdown guide")
        .open(open)
        .resizable(false)
        .show(ctx, |ui| {
            ui.label(RichText::new("A few useful patterns").strong().color(ROSE));
            ui.add_space(8.0);
            for (syntax, description) in [
                ("# Heading", "section title"),
                ("**bold**", "emphasis"),
                ("`code`", "inline code"),
                ("- item", "a list"),
                ("> quote", "a quote"),
                ("```", "code block"),
            ] {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(syntax).monospace().color(FOAM));
                    ui.label(RichText::new(description).color(SUBTLE));
                });
            }
            ui.add_space(10.0);
            ui.label(
                RichText::new("Ctrl/Cmd + O to open · Ctrl/Cmd + S to save")
                    .size(11.0)
                    .color(MUTED),
            );
        });
}

fn display_name(path: &std::path::Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Untitled")
        .to_owned()
}

fn read_markdown_file(path: &std::path::Path) -> Result<(String, String), std::io::Error> {
    let bytes = fs::read(path)?;

    if bytes.starts_with(&[0xFF, 0xFE]) {
        let (text, _, _) = UTF_16LE.decode(&bytes[2..]);
        return Ok((text.into_owned(), "UTF-16 LE".into()));
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        let (text, _, _) = UTF_16BE.decode(&bytes[2..]);
        return Ok((text.into_owned(), "UTF-16 BE".into()));
    }
    if let Ok(text) = String::from_utf8(bytes.clone()) {
        return Ok((
            text.trim_start_matches('\u{FEFF}').to_owned(),
            "UTF-8".into(),
        ));
    }

    let mut detector = EncodingDetector::new();
    detector.feed(&bytes, true);
    let encoding = detector.guess(None, false);
    let (text, _, _) = encoding.decode(&bytes);
    Ok((text.into_owned(), encoding.name().into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_utf16_little_endian_with_bom() {
        let bytes = [0xFF, 0xFE, b'H', 0, b'i', 0];
        let (text, _, _) = UTF_16LE.decode(&bytes[2..]);
        assert_eq!(text, "Hi");
    }

    #[test]
    fn parses_markdown_table() {
        let lines = [
            "| Name | Status |",
            "| :--- | ---: |",
            "| Preview | Ready |",
            "after table",
        ];
        let (headers, rows, next) = parse_table(&lines, 0).expect("valid table");
        assert_eq!(headers, ["Name", "Status"]);
        assert_eq!(rows, [["Preview", "Ready"]]);
        assert_eq!(next, 3);
    }

    #[test]
    fn plain_pipe_text_is_not_a_table() {
        let lines = ["alpha | beta", "not a separator"];
        assert!(parse_table(&lines, 0).is_none());
    }
}
