//! Shared UI components.

use eframe::egui::{self, Color32, CornerRadius, Response, RichText, Sense, StrokeKind, Ui, Vec2};

/// Render a clickable dashboard card with dynamic size.
///
/// Returns the response which can be checked for `.clicked()`.
pub fn dashboard_card(ui: &mut Ui, title: &str, description: &str, icon: &str, size: egui::Vec2) -> Response {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());

    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);

        // Scale factor based on width (200 is the reference size)
        let scale = size.x / 200.0;

        // Card background
        ui.painter().rect_filled(rect, 8.0, visuals.bg_fill);
        ui.painter()
            .rect_stroke(rect, 8.0, visuals.bg_stroke, StrokeKind::Outside);

        // Icon (top area)
        let icon_pos = egui::pos2(rect.center().x, rect.top() + size.y * 0.23);
        ui.painter().text(
            icon_pos,
            egui::Align2::CENTER_CENTER,
            icon,
            egui::FontId::proportional(36.0 * scale),
            visuals.text_color(),
        );

        // Title (middle)
        let title_pos = egui::pos2(rect.center().x, rect.center().y + size.y * 0.07);
        ui.painter().text(
            title_pos,
            egui::Align2::CENTER_CENTER,
            title,
            egui::FontId::proportional(18.0 * scale),
            visuals.text_color(),
        );

        // Description (bottom)
        let desc_pos = egui::pos2(rect.center().x, rect.bottom() - size.y * 0.17);
        ui.painter().text(
            desc_pos,
            egui::Align2::CENTER_CENTER,
            description,
            egui::FontId::proportional(12.0 * scale),
            ui.visuals().weak_text_color(),
        );
    }

    response
}

/// Status indicator colors.
pub mod colors {
    use super::Color32;

    pub const SUCCESS: Color32 = Color32::from_rgb(100, 200, 100);
    pub const ERROR: Color32 = Color32::from_rgb(255, 100, 100);
    pub const WARNING: Color32 = Color32::from_rgb(255, 200, 100);
    pub const NEUTRAL: Color32 = Color32::from_rgb(150, 150, 150);
}

/// Render a back button that returns true when clicked.
pub fn back_button(ui: &mut Ui) -> bool {
    ui.button(RichText::new("< Back to Dashboard").size(14.0)).clicked()
}

/// Render a panel header with title.
pub fn panel_header(ui: &mut Ui, title: &str) {
    ui.heading(RichText::new(title).size(24.0));
    ui.add_space(10.0);
    ui.separator();
    ui.add_space(20.0);
}

/// Button style constants.
pub mod button_style {
    use super::*;

    pub const CORNER_RADIUS: u8 = 6;
    pub const MIN_SIZE: Vec2 = Vec2::new(80.0, 32.0);
    pub const SMALL_MIN_SIZE: Vec2 = Vec2::new(60.0, 26.0);

    pub const PRIMARY_COLOR: Color32 = Color32::from_rgb(70, 130, 200);
    pub const DANGER_COLOR: Color32 = Color32::from_rgb(200, 80, 80);
}

/// Render a styled button with rounded corners and padding.
pub fn styled_button(ui: &mut Ui, text: &str) -> Response {
    let button = egui::Button::new(RichText::new(text).size(14.0))
        .corner_radius(CornerRadius::same(button_style::CORNER_RADIUS))
        .min_size(button_style::MIN_SIZE);
    ui.add(button)
}

/// Render a styled button with icon and text.
pub fn styled_button_with_icon(ui: &mut Ui, icon: &str, text: &str) -> Response {
    let label = format!("{} {}", icon, text);
    let button = egui::Button::new(RichText::new(label).size(14.0))
        .corner_radius(CornerRadius::same(button_style::CORNER_RADIUS))
        .min_size(button_style::MIN_SIZE);
    ui.add(button)
}

/// Render a primary action button (e.g., Save, Add).
pub fn primary_button(ui: &mut Ui, text: &str) -> Response {
    let button = egui::Button::new(RichText::new(text).size(14.0).color(Color32::WHITE))
        .fill(button_style::PRIMARY_COLOR)
        .corner_radius(CornerRadius::same(button_style::CORNER_RADIUS))
        .min_size(button_style::MIN_SIZE);
    ui.add(button)
}

/// Render a primary button with icon.
pub fn primary_button_with_icon(ui: &mut Ui, icon: &str, text: &str) -> Response {
    let label = format!("{} {}", icon, text);
    let button = egui::Button::new(RichText::new(label).size(14.0).color(Color32::WHITE))
        .fill(button_style::PRIMARY_COLOR)
        .corner_radius(CornerRadius::same(button_style::CORNER_RADIUS))
        .min_size(button_style::MIN_SIZE);
    ui.add(button)
}

/// Render a danger button (e.g., Delete).
pub fn danger_button(ui: &mut Ui, text: &str) -> Response {
    let button = egui::Button::new(RichText::new(text).size(14.0).color(Color32::WHITE))
        .fill(button_style::DANGER_COLOR)
        .corner_radius(CornerRadius::same(button_style::CORNER_RADIUS))
        .min_size(button_style::MIN_SIZE);
    ui.add(button)
}

/// Render a small action button with icon and text (for table actions).
pub fn action_button(ui: &mut Ui, icon: &str, text: &str) -> Response {
    let label = format!("{} {}", icon, text);
    let button = egui::Button::new(RichText::new(label).size(12.0))
        .corner_radius(CornerRadius::same(button_style::CORNER_RADIUS))
        .min_size(button_style::SMALL_MIN_SIZE);
    ui.add(button)
}

/// Render a small danger action button with icon and text (for delete actions).
pub fn danger_action_button(ui: &mut Ui, icon: &str, text: &str) -> Response {
    let label = format!("{} {}", icon, text);
    let button = egui::Button::new(RichText::new(label).size(12.0).color(button_style::DANGER_COLOR))
        .corner_radius(CornerRadius::same(button_style::CORNER_RADIUS))
        .min_size(button_style::SMALL_MIN_SIZE);
    ui.add(button)
}
