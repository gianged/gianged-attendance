//! Dashboard panel with navigation cards.

use eframe::egui::{self, RichText, Ui};
use egui_phosphor::regular::{ARROWS_CLOCKWISE, BUILDINGS, USERS};

use super::components::dashboard_card;
use super::main_app::CurrentPanel;

/// Show the dashboard panel.
///
/// Returns `Some(panel)` if a card was clicked to navigate.
pub fn show(ui: &mut Ui) -> Option<CurrentPanel> {
    let mut next_panel = None;

    ui.vertical_centered(|ui| {
        ui.add_space(60.0);

        ui.label(RichText::new("Gianged Attendance").size(32.0).strong());
        ui.add_space(10.0);
        ui.label(RichText::new("Staff and Attendance Management").size(14.0).weak());

        ui.add_space(60.0);

        // Calculate responsive card size
        let available = ui.available_width();
        let num_cards = 3.0;
        let spacing = 30.0;
        let total_spacing = spacing * (num_cards - 1.0);

        // Card width adapts to available space with min/max constraints
        let card_width = ((available - total_spacing) / num_cards).clamp(150.0, 250.0);
        let card_height = card_width * 0.75; // 4:3 aspect ratio
        let card_size = egui::vec2(card_width, card_height);

        // Calculate centering offset
        let total_width = card_width * num_cards + total_spacing;
        let start_offset = ((available - total_width) / 2.0).max(0.0);

        ui.horizontal(|ui| {
            ui.add_space(start_offset);

            if dashboard_card(ui, "Manage Departments", "Organize staff groups", BUILDINGS, card_size).clicked() {
                next_panel = Some(CurrentPanel::Departments);
            }

            ui.add_space(spacing);

            if dashboard_card(ui, "Manage Staff", "Employee records", USERS, card_size).clicked() {
                next_panel = Some(CurrentPanel::Staff);
            }

            ui.add_space(spacing);

            if dashboard_card(ui, "Device Sync", "Sync attendance data", ARROWS_CLOCKWISE, card_size).clicked() {
                next_panel = Some(CurrentPanel::Sync);
            }
        });
    });

    next_panel
}
