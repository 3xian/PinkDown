use std::{path::PathBuf, time::Duration};

use eframe::egui::{self, Color32, FontFamily, FontId, RichText, TextFormat};
use egui_commonmark::CommonMarkCache;

use crate::{
    dialog::{self, Choice},
    document::{pick_markdown_file, Document},
    preview,
    theme::{self, BASE, FOAM, GOLD, HIGHLIGHT_LOW, IRIS, MUTED, ROSE, SUBTLE, SURFACE, TEXT},
    update::{PollResult, UpdateChecker, UpdateOutcome},
};

#[cfg(target_os = "windows")]
use crate::theme::LOVE;

#[cfg(target_os = "macos")]
const SHORTCUT_MOD: &str = "Cmd";
#[cfg(not(target_os = "macos"))]
const SHORTCUT_MOD: &str = "Ctrl";

pub struct PinkDown {
    document: Document,
    status: String,
    pending_action: Option<PendingAction>,
    allow_close: bool,
    markdown_cache: CommonMarkCache,
    update_checker: UpdateChecker,
    update_staged: bool,
    current_title: String,
    #[cfg(target_os = "windows")]
    native_frame_passes: u8,
}

enum PendingAction {
    OpenDialog,
    OpenPath(PathBuf),
    Close,
    #[cfg(target_os = "windows")]
    Restart,
}

impl PendingAction {
    fn confirmation_text(&self) -> &'static str {
        match self {
            Self::OpenDialog | Self::OpenPath(_) => {
                "Save your changes before opening another document?"
            }
            Self::Close => "Save your changes before closing PinkDown?",
            #[cfg(target_os = "windows")]
            Self::Restart => "Save your changes before restarting to install the update?",
        }
    }
}

impl PinkDown {
    pub fn new(cc: &eframe::CreationContext<'_>, initial_path: Option<PathBuf>) -> Self {
        theme::configure(&cc.egui_ctx);
        let mut app = Self {
            document: Document::default(),
            status: "Ready to write".into(),
            pending_action: None,
            allow_close: false,
            markdown_cache: CommonMarkCache::default(),
            update_checker: UpdateChecker::default(),
            update_staged: false,
            current_title: "PinkDown".into(),
            #[cfg(target_os = "windows")]
            native_frame_passes: 0,
        };
        if let Some(path) = initial_path {
            app.open_path(path);
        }
        app
    }

    fn request_action(&mut self, action: PendingAction, ctx: &egui::Context) {
        if self.document.is_dirty() {
            self.pending_action = Some(action);
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        } else {
            self.execute_action(action, ctx);
        }
    }

    fn execute_action(&mut self, action: PendingAction, ctx: &egui::Context) {
        match action {
            PendingAction::OpenDialog => {
                if let Some(path) = pick_markdown_file() {
                    self.open_path(path);
                }
            }
            PendingAction::OpenPath(path) => self.open_path(path),
            PendingAction::Close => {
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            #[cfg(target_os = "windows")]
            PendingAction::Restart => {
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    fn open_path(&mut self, path: PathBuf) {
        match Document::load(path) {
            Ok(document) => {
                let name = document.display_name();
                let encoding = document.encoding_label();
                self.document = document;
                self.markdown_cache = CommonMarkCache::default();
                self.status = format!("Opened {name} · {encoding}");
            }
            Err(error) => self.status = error,
        }
    }

    fn save(&mut self, force_dialog: bool) -> bool {
        match self.document.save(force_dialog) {
            Ok(true) => {
                self.status = format!("Saved {}", self.document.display_name());
                true
            }
            Ok(false) => false,
            Err(error) => {
                self.status = error;
                false
            }
        }
    }

    fn check_close_request(&mut self, ctx: &egui::Context) {
        let close_requested = ctx.input(|input| input.viewport().close_requested());
        if close_requested && !self.allow_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            if self.pending_action.is_none() {
                self.request_action(PendingAction::Close, ctx);
            }
        }
    }

    fn sync_window_title(&mut self, ctx: &egui::Context) {
        let title = self
            .document
            .window_title()
            .unwrap_or_else(|| "PinkDown".to_owned());
        if title != self.current_title {
            self.current_title = title;
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.current_title.clone()));
        }
    }

    fn poll_update(&mut self, ctx: &egui::Context) {
        match self.update_checker.poll() {
            PollResult::Idle => {}
            PollResult::Pending => ctx.request_repaint_after(Duration::from_millis(100)),
            PollResult::Ready(Err(error)) => self.status = format!("Update failed: {error}"),
            PollResult::Ready(Ok(UpdateOutcome::UpToDate(version))) => {
                self.status = format!("PinkDown v{version} is up to date");
            }
            #[cfg(target_os = "windows")]
            PollResult::Ready(Ok(UpdateOutcome::InstallReady(version))) => {
                self.update_staged = true;
                self.status =
                    format!("PinkDown v{version} is staged and will install when PinkDown closes");
                self.request_action(PendingAction::Restart, ctx);
            }
            #[cfg(not(target_os = "windows"))]
            PollResult::Ready(Ok(UpdateOutcome::ManualUpdate(version))) => {
                self.status = format!("PinkDown v{version} is available from GitHub Releases");
            }
        }
    }

    fn show_confirmation(&mut self, ctx: &egui::Context) {
        let Some(action) = self.pending_action.as_ref() else {
            return;
        };
        let message = action.confirmation_text();
        let document_name = self.document.display_name();
        let choice = dialog::unsaved_changes(ctx, &document_name, message);

        match choice {
            Some(Choice::Save) if self.save(false) => {
                if let Some(action) = self.pending_action.take() {
                    self.execute_action(action, ctx);
                }
            }
            Some(Choice::Discard) => {
                if let Some(action) = self.pending_action.take() {
                    self.execute_action(action, ctx);
                }
            }
            Some(Choice::Cancel) => self.pending_action = None,
            _ => {}
        }
    }
}

impl eframe::App for PinkDown {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        #[cfg(target_os = "windows")]
        if self.native_frame_passes < 4 && crate::window::configure_native_window(frame) {
            self.native_frame_passes += 1;
            ctx.request_repaint_after(Duration::from_millis(80));
        }
        #[cfg(not(target_os = "windows"))]
        let _ = frame;

        self.check_close_request(ctx);
        self.poll_update(ctx);
        self.handle_inputs(ctx);
        self.sync_window_title(ctx);
        paint_window_shell(ctx);
        self.show_toolbar(ctx);
        self.show_statusbar(ctx);
        self.show_editor(ctx);

        self.show_confirmation(ctx);
    }
}

impl PinkDown {
    fn handle_inputs(&mut self, ctx: &egui::Context) {
        if ctx.input(|input| input.modifiers.command && input.key_pressed(egui::Key::O)) {
            self.request_action(PendingAction::OpenDialog, ctx);
        }
        if ctx.input(|input| input.modifiers.command && input.key_pressed(egui::Key::S)) {
            let force_dialog = ctx.input(|input| input.modifiers.shift);
            self.save(force_dialog);
        }
        if let Some(path) = ctx
            .input(|input| input.raw.dropped_files.clone())
            .into_iter()
            .find_map(|file| file.path)
        {
            self.request_action(PendingAction::OpenPath(path), ctx);
        }
    }

    fn show_toolbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("window-toolbar")
            .exact_height(64.0)
            .show_separator_line(false)
            .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(20, 10)))
            .show(ctx, |ui| {
                #[cfg(target_os = "windows")]
                configure_title_drag(ui, ctx);

                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        ui.add_space(4.0);
                        gradient_label(ui, "PinkDown", 14.0);
                        ui.label(RichText::new("MARKDOWN STUDIO").size(8.0).color(MUTED));
                    });
                    ui.add_space(16.0);

                    if toolbar_button(ui, "Open", 52.0)
                        .on_hover_text(format!("Open a Markdown file  ({SHORTCUT_MOD}+O)"))
                        .clicked()
                    {
                        self.request_action(PendingAction::OpenDialog, ctx);
                    }
                    if toolbar_button(ui, "Save", 52.0)
                        .on_hover_text(format!("Save the current document  ({SHORTCUT_MOD}+S)"))
                        .clicked()
                    {
                        self.save(false);
                    }
                    if toolbar_button(ui, "Save as", 64.0)
                        .on_hover_text(format!(
                            "Save the document under a new name  ({SHORTCUT_MOD}+Shift+S)"
                        ))
                        .clicked()
                    {
                        self.save(true);
                    }
                    if toolbar_button(ui, "Check updates", 96.0)
                        .on_hover_text("Check GitHub Releases for a newer version")
                        .clicked()
                    {
                        if self.update_staged {
                            self.status =
                                "The staged update will install when PinkDown closes".into();
                        } else if self.update_checker.start() {
                            self.status = "Checking for updates…".into();
                        }
                    }

                    #[cfg(target_os = "windows")]
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        window_controls(ui, ctx, self);
                    });
                });
            });
    }

    fn show_statusbar(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("statusbar")
            .exact_height(36.0)
            .show_separator_line(false)
            .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(8, 4)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(14.0);
                    ui.label(
                        RichText::new(self.document.display_name())
                            .size(12.0)
                            .color(if self.document.is_dirty() {
                                GOLD
                            } else {
                                SUBTLE
                            }),
                    );
                    ui.label(RichText::new(&self.status).size(11.0).color(MUTED));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(14.0);
                        ui.label(
                            RichText::new(format!(
                                "{} words",
                                self.document.text.split_whitespace().count()
                            ))
                            .size(11.0)
                            .color(MUTED),
                        );
                        ui.label(
                            RichText::new(format!("{} lines", self.document.text.lines().count()))
                                .size(11.0)
                                .color(MUTED),
                        );
                    });
                });
            });
    }

    fn show_editor(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(20, 8)))
            .show(ctx, |ui| {
                let available = ui.available_width();
                ui.columns(2, |columns| {
                    columns[0].set_width((available - 12.0) * 0.5);
                    source_panel(&mut columns[0], &mut self.document.text);
                    preview::panel(
                        &mut columns[1],
                        &self.document.text,
                        &mut self.markdown_cache,
                    );
                });
            });
    }
}

fn paint_window_shell(ctx: &egui::Context) {
    let painter = ctx.layer_painter(egui::LayerId::background());
    painter.rect_filled(ctx.screen_rect(), 0.0, BASE);
}

fn toolbar_button(ui: &mut egui::Ui, label: &str, width: f32) -> egui::Response {
    ui.scope(|ui| {
        let widgets = &mut ui.style_mut().visuals.widgets;
        for visuals in [
            &mut widgets.inactive,
            &mut widgets.hovered,
            &mut widgets.active,
        ] {
            visuals.bg_fill = Color32::TRANSPARENT;
            visuals.weak_bg_fill = Color32::TRANSPARENT;
            visuals.bg_stroke = egui::Stroke::NONE;
        }
        widgets.inactive.fg_stroke.color = SUBTLE;
        widgets.hovered.fg_stroke.color = FOAM;
        widgets.active.fg_stroke.color = IRIS;

        ui.add_sized(
            [width, 30.0],
            egui::Button::new(RichText::new(label).size(12.0))
                .frame(true)
                .frame_when_inactive(false),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand)
    })
    .inner
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
                font_id: FontId::new(size, FontFamily::Monospace),
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

fn source_panel(ui: &mut egui::Ui, source: &mut String) {
    egui::Frame::new()
        .fill(SURFACE)
        .stroke(egui::Stroke::new(1.0_f32, HIGHLIGHT_LOW))
        .corner_radius(egui::CornerRadius::same(12))
        .inner_margin(egui::Margin::symmetric(18, 16))
        .show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            ui.horizontal(|ui| {
                ui.label(RichText::new("SOURCE").size(11.0).strong().color(MUTED));
                ui.label(RichText::new("MARKDOWN").size(10.0).color(MUTED));
            });
            ui.add_space(8.0);
            egui::ScrollArea::vertical()
                .id_salt("source-scroll")
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.add_sized(
                        [ui.available_width(), ui.available_height().max(200.0)],
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
}

#[cfg(target_os = "windows")]
fn configure_title_drag(ui: &mut egui::Ui, ctx: &egui::Context) {
    let drag = ui.interact(
        ui.max_rect(),
        ui.id().with("title-drag"),
        egui::Sense::drag(),
    );
    if drag.drag_started() {
        ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
    if drag.double_clicked() {
        let maximized = ctx.input(|input| input.viewport().maximized.unwrap_or(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
    }
}

#[cfg(target_os = "windows")]
#[derive(Clone, Copy)]
enum WindowButton {
    Minimize,
    Maximize,
    Restore,
    Close,
}

#[cfg(target_os = "windows")]
fn window_controls(ui: &mut egui::Ui, ctx: &egui::Context, app: &mut PinkDown) {
    if window_button(ui, WindowButton::Close, "Close").clicked() {
        app.request_action(PendingAction::Close, ctx);
    }
    let maximized = ctx.input(|input| input.viewport().maximized.unwrap_or(false));
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
}

#[cfg(target_os = "windows")]
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
    let stroke = egui::Stroke::new(1.3_f32, color);
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
