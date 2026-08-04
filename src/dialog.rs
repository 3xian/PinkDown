use eframe::egui::{self, Color32, FontFamily, FontId, RichText};

use crate::theme::{
    BASE, GOLD, HIGHLIGHT_LOW, HIGHLIGHT_MED, LOVE, PINE, ROSE, SUBTLE, SURFACE, TEXT,
};

#[derive(Clone, Copy)]
pub enum Choice {
    Save,
    Discard,
    Cancel,
}

pub fn unsaved_changes(ctx: &egui::Context, document_name: &str, message: &str) -> Option<Choice> {
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
            heading(ui, document_name);
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

fn heading(ui: &mut egui::Ui, document_name: &str) {
    ui.horizontal_top(|ui| {
        let (icon_rect, _) = ui.allocate_exact_size(egui::vec2(38.0, 38.0), egui::Sense::hover());
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
                egui::Stroke::new(1.0, LOVE.gamma_multiply(0.55)),
            ),
            ButtonKind::Secondary => (
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
