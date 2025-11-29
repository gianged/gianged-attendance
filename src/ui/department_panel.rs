//! Department management panel with full CRUD functionality.

use eframe::egui::{self, ScrollArea, Ui};
use egui_phosphor::regular::{ARROWS_CLOCKWISE, PENCIL, PLUS, TRASH};

use super::app::{App, DeleteTarget, DepartmentForm};
use super::components::{
    action_button, back_button, danger_action_button, panel_header, primary_button_with_icon, styled_button,
    styled_button_with_icon,
};
use crate::models::department::{CreateDepartment, UpdateDepartment};

/// Show the department panel.
///
/// Returns `true` if the back button was clicked.
pub fn show(app: &mut App, ui: &mut Ui) -> bool {
    let mut go_back = false;

    if back_button(ui) {
        go_back = true;
    }

    panel_header(ui, "Manage Departments");

    // Toolbar
    ui.horizontal(|ui| {
        if primary_button_with_icon(ui, PLUS, "Add Department").clicked() {
            app.department_form = DepartmentForm {
                is_active: true,
                is_open: true,
                display_order: "0".to_string(),
                ..Default::default()
            };
        }

        ui.add_space(10.0);

        if styled_button_with_icon(ui, ARROWS_CLOCKWISE, "Refresh").clicked() {
            app.load_departments();
        }
    });

    ui.add_space(15.0);

    // Department count
    ui.label(format!("{count} departments", count = app.departments.len()));

    ui.add_space(10.0);

    // Table
    show_table(app, ui);

    // Form dialog
    if app.department_form.is_open {
        show_form_dialog(app, ui.ctx());
    }

    go_back
}

fn show_table(app: &mut App, ui: &mut Ui) {
    ScrollArea::vertical().show(ui, |ui| {
        egui::Grid::new("departments_grid")
            .num_columns(6)
            .striped(true)
            .min_col_width(80.0)
            .spacing([15.0, 8.0])
            .show(ui, |ui| {
                // Header
                ui.strong("ID");
                ui.strong("Name");
                ui.strong("Parent");
                ui.strong("Order");
                ui.strong("Active");
                ui.strong("Actions");
                ui.end_row();

                // Data rows
                let departments = app.departments.clone();
                for dept in &departments {
                    ui.label(dept.id.to_string());
                    ui.label(&dept.name);

                    // Parent name
                    let parent_name = dept
                        .parent_id
                        .and_then(|pid| app.departments.iter().find(|d| d.id == pid))
                        .map(|d| d.name.as_str())
                        .unwrap_or("-");
                    ui.label(parent_name);

                    ui.label(dept.display_order.to_string());
                    ui.label(if dept.is_active { "Yes" } else { "No" });

                    ui.horizontal(|ui| {
                        if action_button(ui, PENCIL, "Edit").clicked() {
                            app.department_form = DepartmentForm::edit(dept);
                        }
                        if danger_action_button(ui, TRASH, "Delete").clicked() {
                            app.delete_target = Some(DeleteTarget::Department(dept.id, dept.name.clone()));
                            app.show_delete_confirm = true;
                        }
                    });

                    ui.end_row();
                }
            });
    });
}

fn show_form_dialog(app: &mut App, ctx: &egui::Context) {
    let title = if app.department_form.is_editing {
        "Edit Department"
    } else {
        "Add Department"
    };

    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .default_width(400.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.add_space(10.0);

            egui::Grid::new("dept_form_grid")
                .num_columns(2)
                .spacing([20.0, 12.0])
                .show(ui, |ui| {
                    ui.label("Name:");
                    ui.add(egui::TextEdit::singleline(&mut app.department_form.name).desired_width(250.0));
                    ui.end_row();

                    ui.label("Parent:");
                    egui::ComboBox::from_id_salt("dept_parent")
                        .width(250.0)
                        .selected_text(
                            app.department_form
                                .parent_id
                                .and_then(|id| app.departments.iter().find(|d| d.id == id))
                                .map(|d| d.name.as_str())
                                .unwrap_or("None"),
                        )
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(app.department_form.parent_id.is_none(), "None")
                                .clicked()
                            {
                                app.department_form.parent_id = None;
                            }

                            for dept in &app.departments {
                                // Skip self to prevent circular reference
                                if Some(dept.id) == app.department_form.id {
                                    continue;
                                }

                                if ui
                                    .selectable_label(app.department_form.parent_id == Some(dept.id), &dept.name)
                                    .clicked()
                                {
                                    app.department_form.parent_id = Some(dept.id);
                                }
                            }
                        });
                    ui.end_row();

                    ui.label("Display Order:");
                    ui.add(egui::TextEdit::singleline(&mut app.department_form.display_order).desired_width(100.0));
                    ui.end_row();

                    ui.label("Active:");
                    ui.checkbox(&mut app.department_form.is_active, "");
                    ui.end_row();
                });

            ui.add_space(15.0);
            ui.separator();
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                if styled_button(ui, "Cancel").clicked() {
                    app.department_form.reset();
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if primary_button_with_icon(ui, "", "Save").clicked() {
                        save_department(app);
                    }
                });
            });
        });
}

fn save_department(app: &mut App) {
    let form = &app.department_form;

    // Validation
    if form.name.trim().is_empty() {
        app.error_message = Some("Name is required".to_string());
        return;
    }

    let display_order = form.display_order.parse().unwrap_or(0);

    if form.is_editing {
        // Update
        let id = form.id.unwrap();
        let data = UpdateDepartment {
            name: Some(form.name.clone()),
            parent_id: Some(form.parent_id),
            display_order: Some(display_order),
            is_active: Some(form.is_active),
        };
        app.update_department(id, data);
    } else {
        // Create
        let data = CreateDepartment {
            name: form.name.clone(),
            parent_id: form.parent_id,
            display_order,
        };
        app.create_department(data);
    }
}
