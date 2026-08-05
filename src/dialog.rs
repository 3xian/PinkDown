use eframe::egui::{self, Color32, FontFamily, FontId, RichText};

use crate::theme::{
    self, BASE, FOAM, GOLD, HIGHLIGHT_LOW, HIGHLIGHT_MED, LOVE, MUTED, PINE, ROSE, SUBTLE, SURFACE,
    TEXT, FONT_AUTO,
};

#[derive(Clone, Copy)]
pub enum Choice {
    Save,
    Discard,
    Cancel,
}

pub fn unsaved_changes(ctx: &egui::Context, document_name: &str, message: &str) -> Option<Choice> {
    let modal = egui::Modal::new(egui::Id::new("unsaved-changes-modal"))
        .backdrop_color(Color32::from_black_alpha(150))
        .frame(modal_frame())
        .show(ctx, |ui| {
            ui.set_width(390.0);
            dialog_heading(
                ui,
                "!",
                18.0,
                GOLD,
                "Unsaved changes",
                &format!("“{document_name}” has edits not yet saved."),
            );
            ui.add_space(18.0);
            message_panel(ui, message);
            ui.add_space(20.0);

            let mut choice = buttons(ui);
            if ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) {
                choice = Some(Choice::Save);
            }
            choice
        });

    let should_cancel = modal.should_close();
    modal.inner.or(should_cancel.then_some(Choice::Cancel))
}

fn message_panel(ui: &mut egui::Ui, message: &str) {
    egui::Frame::new()
        .fill(BASE)
        .corner_radius(egui::CornerRadius::same(10))
        .inner_margin(egui::Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(message).size(13.0).color(SUBTLE));
        });
}

fn buttons(ui: &mut egui::Ui) -> Option<Choice> {
    let mut choice = None;
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        if button(ui, "Save changes", 112.0, ButtonKind::Primary).clicked() {
            choice = Some(Choice::Save);
        }
        if button(ui, "Discard", 88.0, ButtonKind::Danger).clicked() {
            choice = Some(Choice::Discard);
        }
        if button(ui, "Cancel", 76.0, ButtonKind::Secondary).clicked() {
            choice = Some(Choice::Cancel);
        }
    });
    choice
}

#[derive(Clone, Copy)]
enum ButtonKind {
    Primary,
    Danger,
    Secondary,
}

fn button(ui: &mut egui::Ui, label: &str, width: f32, kind: ButtonKind) -> egui::Response {
    ui.scope(|ui| {
        let visuals = &mut ui.style_mut().visuals.widgets;
        let (inactive_fill, hovered_fill, active_fill, text_color, stroke) = match kind {
            ButtonKind::Primary => (
                PINE,
                Color32::from_rgb(58, 135, 162),
                Color32::from_rgb(42, 101, 126),
                TEXT,
                egui::Stroke::NONE,
            ),
            ButtonKind::Danger => (
                HIGHLIGHT_LOW,
                LOVE.gamma_multiply(0.28),
                LOVE.gamma_multiply(0.4),
                ROSE,
                egui::Stroke::new(1.0_f32, LOVE.gamma_multiply(0.55)),
            ),
            ButtonKind::Secondary => (
                Color32::TRANSPARENT,
                HIGHLIGHT_MED,
                HIGHLIGHT_LOW,
                SUBTLE,
                egui::Stroke::new(1.0_f32, HIGHLIGHT_MED),
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

fn modal_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(SURFACE)
        .stroke(egui::Stroke::new(1.0_f32, HIGHLIGHT_MED))
        .corner_radius(egui::CornerRadius::same(16))
        .inner_margin(egui::Margin::symmetric(26, 24))
        .shadow(egui::Shadow {
            offset: [0, 12],
            blur: 36,
            spread: 0,
            color: Color32::from_black_alpha(150),
        })
}

fn dialog_heading(
    ui: &mut egui::Ui,
    icon: &str,
    icon_size: f32,
    accent: Color32,
    title: &str,
    subtitle: &str,
) {
    ui.horizontal_top(|ui| {
        let (icon_rect, _) = ui.allocate_exact_size(egui::vec2(38.0, 38.0), egui::Sense::hover());
        ui.painter()
            .circle_filled(icon_rect.center(), 19.0, accent.gamma_multiply(0.16));
        ui.painter().circle_stroke(
            icon_rect.center(),
            18.5,
            egui::Stroke::new(1.0_f32, accent.gamma_multiply(0.55)),
        );
        ui.painter().text(
            icon_rect.center(),
            egui::Align2::CENTER_CENTER,
            icon,
            FontId::new(icon_size, FontFamily::Proportional),
            accent,
        );

        ui.add_space(6.0);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            ui.label(RichText::new(title).size(19.0).strong().color(TEXT));
            ui.label(RichText::new(subtitle).size(13.0).color(SUBTLE));
        });
    });
}

/// Choice from the update-available confirmation dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateChoice {
    /// Primary action: install (auto) or open the releases page (manual).
    Update,
    Later,
}

/// How the update prompt should present its primary action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdatePromptMode {
    /// Download, stage, then quit PinkDown so the helper can install.
    InstallAndRestart,
    /// Open the GitHub Releases page for a manual install.
    OpenReleases,
}

/// Asks whether to proceed with a newer PinkDown release.
pub fn update_available(
    ctx: &egui::Context,
    current_version: &str,
    latest_version: &str,
    mode: UpdatePromptMode,
) -> Option<UpdateChoice> {
    let modal = egui::Modal::new(egui::Id::new("update-available-modal"))
        .backdrop_color(Color32::from_black_alpha(150))
        .frame(modal_frame())
        .show(ctx, |ui| {
            ui.set_width(390.0);
            let (subtitle, body, primary_label, primary_width) = match mode {
                UpdatePromptMode::InstallAndRestart => (
                    format!("PinkDown v{latest_version} is ready to install."),
                    format!(
                        "You are running v{current_version}. Update downloads the package now; \
                         PinkDown will quit afterward so installation can finish, then relaunch."
                    ),
                    "Update",
                    96.0_f32,
                ),
                UpdatePromptMode::OpenReleases => (
                    format!("PinkDown v{latest_version} is available on GitHub."),
                    format!(
                        "You are running v{current_version}. Open the releases page to download \
                         it, or choose Later to keep this version."
                    ),
                    "Open Releases",
                    124.0_f32,
                ),
            };
            dialog_heading(
                ui,
                "\u{2191}",
                18.0,
                FOAM,
                "Update available",
                &subtitle,
            );
            ui.add_space(18.0);
            message_panel(ui, &body);
            ui.add_space(20.0);

            let mut choice = None;
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if button(ui, primary_label, primary_width, ButtonKind::Primary).clicked()
                    || ui.input_mut(|input| {
                        input.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
                    })
                {
                    choice = Some(UpdateChoice::Update);
                }
                if button(ui, "Later", 76.0, ButtonKind::Secondary).clicked() {
                    choice = Some(UpdateChoice::Later);
                }
            });
            choice
        });

    let should_cancel = modal.should_close();
    modal.inner.or(should_cancel.then_some(UpdateChoice::Later))
}

/// Result of the font settings modal for one frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontSettingsAction {
    /// Keep the dialog open; selection may have changed in `draft`.
    KeepOpen,
    /// Persist `draft` and close.
    Apply,
    /// Discard and close.
    Cancel,
}

/// Modal for choosing the preferred UI / preview typeface.
///
/// `preferred_font` is a draft id (`"auto"` or a catalog entry). Preview of the
/// unapplied face is intentionally omitted: egui applies fonts globally, and a
/// label that only shows the *name* would be misleading.
pub fn font_settings(ctx: &egui::Context, preferred_font: &mut String) -> FontSettingsAction {
    let modal = egui::Modal::new(egui::Id::new("font-settings-modal"))
        .backdrop_color(Color32::from_black_alpha(150))
        .frame(modal_frame())
        .show(ctx, |ui| {
            ui.set_width(390.0);
            dialog_heading(
                ui,
                "Aa",
                15.0,
                FOAM,
                "Font",
                "Choose the typeface used for the UI and preview.",
            );
            ui.add_space(18.0);
            font_picker(ui, preferred_font);
            ui.add_space(20.0);

            let mut action = FontSettingsAction::KeepOpen;
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if button(ui, "Apply", 88.0, ButtonKind::Primary).clicked()
                    || ui.input_mut(|input| {
                        input.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
                    })
                {
                    action = FontSettingsAction::Apply;
                }
                if button(ui, "Cancel", 76.0, ButtonKind::Secondary).clicked() {
                    action = FontSettingsAction::Cancel;
                }
            });
            action
        });

    if modal.should_close() {
        FontSettingsAction::Cancel
    } else {
        modal.inner
    }
}

fn font_picker(ui: &mut egui::Ui, preferred_font: &mut String) {
    egui::Frame::new()
        .fill(BASE)
        .corner_radius(egui::CornerRadius::same(10))
        .inner_margin(egui::Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new("TYPEFACE").size(10.0).strong().color(MUTED));
            ui.add_space(8.0);

            let available = theme::available_fonts();
            let current_label = theme::font_label(preferred_font);

            egui::ComboBox::from_id_salt("preferred-font")
                .width(ui.available_width().max(120.0))
                .selected_text(RichText::new(current_label).size(13.0).color(TEXT))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        preferred_font,
                        FONT_AUTO.to_owned(),
                        RichText::new("Auto").size(13.0),
                    );
                    for font in available {
                        ui.selectable_value(
                            preferred_font,
                            font.id.to_owned(),
                            RichText::new(font.label).size(13.0),
                        );
                    }
                });

            ui.add_space(8.0);
            ui.label(
                RichText::new(
                    "Auto keeps the built-in Latin face and uses the first available \
                     system font for CJK glyphs. An explicit choice becomes the main \
                     UI and preview typeface. The source editor stays monospaced for Latin text.",
                )
                .size(11.0)
                .color(SUBTLE),
            );
        });
}
