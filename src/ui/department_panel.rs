//! Department management panel (placeholder).

use eframe::egui::{RichText, Ui};

use super::components::{back_button, panel_header};

/// Show the department panel.
///
/// Returns `true` if the back button was clicked.
pub fn show(ui: &mut Ui) -> bool {
    let mut go_back = false;

    if back_button(ui) {
        go_back = true;
    }

    panel_header(ui, "Manage Departments");

    ui.vertical_centered(|ui| {
        ui.add_space(40.0);
        ui.label(RichText::new("Coming soon").size(18.0).weak());
        ui.add_space(10.0);
        ui.label("Department management functionality will be implemented here.");
    });

    go_back
}
