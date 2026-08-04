use std::{path::PathBuf, time::Duration};

use eframe::egui::{self, Color32, FontFamily, FontId, RichText, TextFormat};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};

use crate::{
    document::{pick_markdown_file, Document},
    theme::{
        self, BASE, CONTENT_FONT_SIZE, FOAM, GOLD, HIGHLIGHT_LOW, HIGHLIGHT_MED, IRIS, LOVE, MUTED,
        PINE, ROSE, SUBTLE, SURFACE, TEXT,
    },
    update::{PollResult, UpdateChecker, UpdateOutcome},
};

pub struct PinkDown {
    document: Document,
    status: String,
    pending_action: Option<PendingAction>,
    allow_close: bool,
    markdown_cache: CommonMarkCache,
    update_checker: UpdateChecker,
    update_staged: bool,
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

#[derive(Clone, Copy)]
enum Confirmation {
    Save,
    Discard,
    Cancel,
}

impl PinkDown {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::configure(&cc.egui_ctx);
        Self {
            document: Document::default(),
            status: "Ready to write".into(),
            pending_action: None,
            allow_close: false,
            markdown_cache: CommonMarkCache::default(),
            update_checker: UpdateChecker::default(),
            update_staged: false,
            #[cfg(target_os = "windows")]
            native_frame_passes: 0,
        }
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
        let frame = egui::Frame::new()
            .fill(SURFACE)
            .stroke(egui::Stroke::new(1.0, HIGHLIGHT_MED))
            .corner_radius(egui::CornerRadius::same(16))
            .inner_margin(egui::Margin::symmetric(26, 24))
            .shadow(egui::Shadow {
                offset: [0, 12],
                blur: 36,
                spread: 0,
                color: Color32::from_black_alpha(150),
            });

        let modal = egui::Modal::new(egui::Id::new("unsaved-changes-modal"))
            .backdrop_color(Color32::from_black_alpha(150))
            .frame(frame)
            .show(ctx, |ui| {
                ui.set_width(390.0);

                ui.horizontal_top(|ui| {
                    let (icon_rect, _) =
                        ui.allocate_exact_size(egui::vec2(38.0, 38.0), egui::Sense::hover());
                    ui.painter()
                        .circle_filled(icon_rect.center(), 19.0, GOLD.gamma_multiply(0.16));
                    ui.painter().circle_stroke(
                        icon_rect.center(),
                        18.5,
                        egui::Stroke::new(1.0, GOLD.gamma_multiply(0.55)),
                    );
                    ui.painter().text(
                        icon_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "!",
                        FontId::new(18.0, FontFamily::Proportional),
                        GOLD,
                    );

                    ui.add_space(6.0);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 6.0;
                        ui.label(
                            RichText::new("Unsaved changes")
                                .size(19.0)
                                .strong()
                                .color(TEXT),
                        );
                        ui.label(
                            RichText::new(format!("“{document_name}” has edits not yet saved."))
                                .size(13.0)
                                .color(SUBTLE),
                        );
                    });
                });

                ui.add_space(18.0);
                egui::Frame::new()
                    .fill(BASE)
                    .corner_radius(egui::CornerRadius::same(10))
                    .inner_margin(egui::Margin::symmetric(14, 12))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new(message).size(13.0).color(SUBTLE));
                    });
                ui.add_space(20.0);

                let mut choice = None;
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if dialog_button(ui, "Save changes", 112.0, DialogButtonKind::Primary).clicked()
                    {
                        choice = Some(Confirmation::Save);
                    }
                    if dialog_button(ui, "Discard", 88.0, DialogButtonKind::Danger).clicked() {
                        choice = Some(Confirmation::Discard);
                    }
                    if dialog_button(ui, "Cancel", 76.0, DialogButtonKind::Secondary).clicked() {
                        choice = Some(Confirmation::Cancel);
                    }
                });

                if ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
                {
                    choice = Some(Confirmation::Save);
                }
                choice
            });

        let should_cancel = modal.should_close();
        let choice = modal
            .inner
            .or(should_cancel.then_some(Confirmation::Cancel));

        match choice {
            Some(Confirmation::Save) if self.save(false) => {
                if let Some(action) = self.pending_action.take() {
                    self.execute_action(action, ctx);
                }
            }
            Some(Confirmation::Discard) => {
                if let Some(action) = self.pending_action.take() {
                    self.execute_action(action, ctx);
                }
            }
            Some(Confirmation::Cancel) => self.pending_action = None,
            _ => {}
        }
    }
}

#[derive(Clone, Copy)]
enum DialogButtonKind {
    Primary,
    Danger,
    Secondary,
}

fn dialog_button(
    ui: &mut egui::Ui,
    label: &str,
    width: f32,
    kind: DialogButtonKind,
) -> egui::Response {
    ui.scope(|ui| {
        let visuals = &mut ui.style_mut().visuals.widgets;
        let (inactive_fill, hovered_fill, active_fill, text_color, stroke) = match kind {
            DialogButtonKind::Primary => (
                PINE,
                Color32::from_rgb(58, 135, 162),
                Color32::from_rgb(42, 101, 126),
                TEXT,
                egui::Stroke::NONE,
            ),
            DialogButtonKind::Danger => (
                HIGHLIGHT_LOW,
                LOVE.gamma_multiply(0.28),
                LOVE.gamma_multiply(0.4),
                ROSE,
                egui::Stroke::new(1.0, LOVE.gamma_multiply(0.55)),
            ),
            DialogButtonKind::Secondary => (
                Color32::TRANSPARENT,
                HIGHLIGHT_MED,
                HIGHLIGHT_LOW,
                SUBTLE,
                egui::Stroke::new(1.0, HIGHLIGHT_MED),
            ),
        };

        visuals.inactive.bg_fill = inactive_fill;
        visuals.inactive.weak_bg_fill = inactive_fill;
        visuals.inactive.bg_stroke = stroke;
        visuals.hovered.bg_fill = hovered_fill;
        visuals.hovered.weak_bg_fill = hovered_fill;
        visuals.hovered.bg_stroke = stroke;
        visuals.active.bg_fill = active_fill;
        visuals.active.weak_bg_fill = active_fill;
        visuals.active.bg_stroke = stroke;

        ui.add_sized(
            [width, 36.0],
            egui::Button::new(RichText::new(label).size(12.0).strong().color(text_color))
                .corner_radius(egui::CornerRadius::same(9)),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand)
    })
    .inner
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
            self.save(false);
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

                    if toolbar_button(ui, "Open", 52.0).clicked() {
                        self.request_action(PendingAction::OpenDialog, ctx);
                    }
                    if toolbar_button(ui, "Save", 52.0).clicked() {
                        self.save(false);
                    }
                    if toolbar_button(ui, "Save as", 64.0).clicked() {
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

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        #[cfg(target_os = "windows")]
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
                    preview_panel(
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
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 30.0), egui::Sense::click());
    let hovered = response.hovered();
    let color = if response.is_pointer_button_down_on() {
        IRIS
    } else if hovered {
        FOAM
    } else {
        SUBTLE
    };
    let font = FontId::new(12.0, FontFamily::Proportional);

    if hovered {
        for offset in [-0.35, 0.35] {
            ui.painter().text(
                rect.center() + egui::vec2(offset, 0.0),
                egui::Align2::CENTER_CENTER,
                label,
                font.clone(),
                color,
            );
        }
    } else {
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            font,
            color,
        );
    }

    response.on_hover_cursor(egui::CursorIcon::PointingHand)
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

fn preview_panel(ui: &mut egui::Ui, source: &str, cache: &mut CommonMarkCache) {
    egui::Frame::new()
        .fill(BASE)
        .stroke(egui::Stroke::new(1.0_f32, HIGHLIGHT_LOW))
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
                        render_preview_markdown(ui, cache, source);
                    });
                    ui.add_space(24.0);
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

fn render_preview_markdown(ui: &mut egui::Ui, cache: &mut CommonMarkCache, source: &str) {
    let lines: Vec<&str> = source.split_inclusive('\n').collect();
    let mut chunk_start = 0;
    let mut offset = 0;
    let mut open_fence: Option<(u8, usize, usize, &str)> = None;
    let mut index = 0;

    while index < lines.len() {
        let line = lines[index];
        if let Some((open_marker, minimum_length, content_start, language)) = open_fence {
            if let Some((marker, length, remainder)) = fenced_code_marker(line) {
                if marker == open_marker && length >= minimum_length && remainder.trim().is_empty()
                {
                    render_preview_code_block(ui, &source[content_start..offset], language);
                    chunk_start = offset + line.len();
                    open_fence = None;
                }
            }
            offset += line.len();
            index += 1;
            continue;
        }

        if let Some((marker, length, remainder)) = fenced_code_marker(line) {
            if chunk_start < offset {
                CommonMarkViewer::new().show(ui, cache, &source[chunk_start..offset]);
            }
            let language = remainder.trim().split_whitespace().next().unwrap_or("");
            open_fence = Some((marker, length, offset + line.len(), language));
            offset += line.len();
            index += 1;
            continue;
        }

        if let Some((headers, table_end)) = markdown_table_at(&lines, index) {
            if chunk_start < offset {
                CommonMarkViewer::new().show(ui, cache, &source[chunk_start..offset]);
            }
            let rows = lines[index + 2..table_end]
                .iter()
                .filter_map(|line| markdown_table_cells(line))
                .collect::<Vec<_>>();
            render_preview_table(ui, cache, &headers, &rows);

            offset += lines[index..table_end]
                .iter()
                .map(|line| line.len())
                .sum::<usize>();
            chunk_start = offset;
            index = table_end;
            continue;
        }

        if let Some(level) = atx_heading_level(line) {
            if chunk_start < offset {
                CommonMarkViewer::new().show(ui, cache, &source[chunk_start..offset]);
            }
            render_colored_heading(ui, cache, line, level);
            chunk_start = offset + line.len();
        }
        offset += line.len();
        index += 1;
    }

    if let Some((_, _, content_start, language)) = open_fence {
        render_preview_code_block(ui, &source[content_start..], language);
    } else if chunk_start < source.len() {
        CommonMarkViewer::new().show(ui, cache, &source[chunk_start..]);
    }
}

fn markdown_table_at<'a>(lines: &[&'a str], start: usize) -> Option<(Vec<&'a str>, usize)> {
    let headers = markdown_table_cells(*lines.get(start)?)?;
    let separator = markdown_table_cells(*lines.get(start + 1)?)?;
    if headers.len() != separator.len() || !separator.iter().all(|cell| markdown_table_rule(cell)) {
        return None;
    }

    let mut end = start + 2;
    while lines
        .get(end)
        .and_then(|line| markdown_table_cells(line))
        .is_some()
    {
        end += 1;
    }
    Some((headers, end))
}

fn markdown_table_cells(line: &str) -> Option<Vec<&str>> {
    let line = line.trim();
    if !line.contains('|') {
        return None;
    }
    let content = line.strip_prefix('|').unwrap_or(line);
    let content = content.strip_suffix('|').unwrap_or(content);
    let cells = content.split('|').map(str::trim).collect::<Vec<_>>();
    (cells.len() >= 2).then_some(cells)
}

fn markdown_table_rule(cell: &str) -> bool {
    let rule = cell.trim().trim_matches(':');
    rule.len() >= 3 && rule.bytes().all(|byte| byte == b'-')
}

fn render_preview_table(
    ui: &mut egui::Ui,
    cache: &mut CommonMarkCache,
    headers: &[&str],
    rows: &[Vec<&str>],
) {
    let column_count = headers.len().max(1);
    let gap = 12.0;
    let cell_width = ((ui.available_width() - gap * (column_count - 1) as f32 - 20.0)
        / column_count as f32)
        .max(72.0);

    ui.add_space(4.0);
    egui::Frame::new()
        .fill(HIGHLIGHT_LOW)
        .corner_radius(egui::CornerRadius::same(7))
        .inner_margin(1.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            render_preview_table_row(ui, cache, headers, cell_width, gap, true, false);
            for (index, row) in rows.iter().enumerate() {
                render_preview_table_row(ui, cache, row, cell_width, gap, false, index % 2 == 1);
            }
        });
    ui.add_space(4.0);
}

fn render_preview_table_row(
    ui: &mut egui::Ui,
    cache: &mut CommonMarkCache,
    cells: &[&str],
    cell_width: f32,
    gap: f32,
    header: bool,
    alternate: bool,
) {
    let fill = if header {
        HIGHLIGHT_MED
    } else if alternate {
        HIGHLIGHT_LOW
    } else {
        SURFACE
    };

    egui::Frame::new()
        .fill(fill)
        .corner_radius(egui::CornerRadius::same(if header { 6 } else { 2 }))
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.spacing_mut().item_spacing.x = gap;
            ui.horizontal_top(|ui| {
                for cell in cells {
                    ui.allocate_ui_with_layout(
                        egui::vec2(cell_width, 0.0),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.set_width(cell_width);
                            if header {
                                let text = format!("**{cell}**");
                                CommonMarkViewer::new().show(ui, cache, &text);
                            } else {
                                CommonMarkViewer::new().show(ui, cache, cell);
                            }
                        },
                    );
                }
            });
        });
}

fn render_preview_code_block(ui: &mut egui::Ui, code: &str, language: &str) {
    let code = code.strip_suffix('\n').unwrap_or(code);
    let code = code.strip_suffix('\r').unwrap_or(code);

    ui.add_space(4.0);
    egui::Frame::new()
        .fill(SURFACE)
        .corner_radius(egui::CornerRadius::same(4))
        .inner_margin(egui::Margin {
            left: 14,
            right: 14,
            top: 6,
            bottom: 12,
        })
        .shadow(egui::Shadow {
            offset: [0, 4],
            blur: 14,
            spread: 0,
            color: Color32::from_black_alpha(85),
        })
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(if language.is_empty() {
                        "CODE"
                    } else {
                        language
                    })
                    .size(10.0)
                    .strong()
                    .color(MUTED),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(RichText::new("Copy").size(10.0).color(SUBTLE))
                                .frame(false),
                        )
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        ui.ctx().copy_text(code.to_owned());
                    }
                });
            });
            ui.add_space(6.0);
            egui::ScrollArea::horizontal()
                .id_salt(("preview-code", code.as_ptr()))
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.add(
                        egui::Label::new(
                            RichText::new(code)
                                .font(FontId::new(CONTENT_FONT_SIZE, FontFamily::Monospace))
                                .color(FOAM),
                        )
                        .selectable(true)
                        .wrap_mode(egui::TextWrapMode::Extend),
                    );
                });
        });
    ui.add_space(4.0);
}

fn fenced_code_marker(line: &str) -> Option<(u8, usize, &str)> {
    let indentation = line.len() - line.trim_start_matches(' ').len();
    if indentation > 3 {
        return None;
    }

    let line = &line[indentation..];
    let marker = *line.as_bytes().first()?;
    if !matches!(marker, b'`' | b'~') {
        return None;
    }
    let length = line.bytes().take_while(|byte| *byte == marker).count();
    (length >= 3).then_some((marker, length, &line[length..]))
}

fn atx_heading_level(line: &str) -> Option<u8> {
    let indentation = line.len() - line.trim_start_matches(' ').len();
    if indentation > 3 {
        return None;
    }

    let heading = &line[indentation..];
    let level = heading.bytes().take_while(|byte| *byte == b'#').count();
    (matches!(level, 1..=3)
        && heading
            .as_bytes()
            .get(level)
            .is_some_and(u8::is_ascii_whitespace))
    .then_some(level as u8)
}

fn render_colored_heading(ui: &mut egui::Ui, cache: &mut CommonMarkCache, source: &str, level: u8) {
    let (color, heading_style_size) = match level {
        1 => (ROSE, 26.0),
        2 => (IRIS, 22.6),
        3 => (FOAM, 19.0),
        _ => unreachable!("only level 1-3 headings are rendered here"),
    };

    ui.scope(|ui| {
        ui.visuals_mut().widgets.active.fg_stroke.color = color;
        ui.style_mut().text_styles.insert(
            egui::TextStyle::Heading,
            FontId::new(heading_style_size, FontFamily::Proportional),
        );
        CommonMarkViewer::new().show(ui, cache, source);
    });
}

#[cfg(test)]
mod preview_tests {
    use super::{markdown_table_at, markdown_table_cells};

    #[test]
    fn parses_wide_chinese_table_with_inline_code() {
        let source = "| 门禁 | 人工必须确认 | Agent 才可以 |\n\
                      |---|---|---|\n\
                      | G1 创作契约 | 是否做、内容支柱、`PLAY-Bxxx`、受众、触发、一个决定、一个承诺、选定 Hook、三拍正文、删减项 | 锁定批次基线、运行 `new_video.py`、核验法律并起草 |";
        let lines = source.split_inclusive('\n').collect::<Vec<_>>();

        let (headers, end) = markdown_table_at(&lines, 0).expect("valid table");
        let row = markdown_table_cells(lines[2]).expect("valid data row");

        assert_eq!(headers, ["门禁", "人工必须确认", "Agent 才可以"]);
        assert_eq!(end, 3);
        assert_eq!(row.len(), 3);
        assert!(row[1].contains("`PLAY-Bxxx`"));
        assert!(row[2].contains("`new_video.py`"));
    }
}
