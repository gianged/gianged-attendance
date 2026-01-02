//! Employee management panel with full CRUD, search, and filter functionality.

use chrono::Local;
use eframe::egui::{self, ScrollArea, Ui};
use egui_phosphor::regular::{ARROWS_CLOCKWISE, FILE_XLS, PENCIL, PLUS, TRASH};

use super::app::{App, DeleteTarget, EmployeeForm};
use super::components::{
    action_button, back_button, colors, danger_action_button, panel_header, primary_button_with_icon, styled_button,
    styled_button_with_icon,
};
use crate::models::employee::{CreateEmployee, UpdateEmployee};

/// Parse date input flexibly, accepting multiple formats.
fn parse_flexible_date(input: &str) -> Option<chrono::NaiveDate> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }

    for fmt in &["%Y-%m-%d", "%Y/%m/%d", "%Y.%m.%d"] {
        if let Ok(date) = chrono::NaiveDate::parse_from_str(input, fmt) {
            return Some(date);
        }
    }
    None
}

/// Show the staff panel.
///
/// Returns `true` if the back button was clicked.
pub fn show(app: &mut App, ui: &mut Ui) -> bool {
    let mut go_back = false;

    if back_button(ui) {
        go_back = true;
    }

    panel_header(ui, "Manage Staff");

    // Toolbar row 1: Action buttons
    ui.horizontal(|ui| {
        if primary_button_with_icon(ui, PLUS, "Add Employee").clicked() {
            let today = Local::now().date_naive();
            app.employee_form = EmployeeForm {
                is_active: true,
                start_date: Some(today),
                start_date_input: today.format("%Y-%m-%d").to_string(),
                is_open: true,
                ..Default::default()
            };
        }

        ui.add_space(10.0);

        if styled_button_with_icon(ui, ARROWS_CLOCKWISE, "Refresh").clicked() {
            app.load_employees();
        }

        ui.add_space(10.0);

        if styled_button_with_icon(ui, FILE_XLS, "Export to Excel").clicked() {
            app.export_employees();
        }
    });

    ui.add_space(10.0);

    // Toolbar row 2: Search and filter
    ui.horizontal(|ui| {
        ui.label("Search:");
        ui.add(
            egui::TextEdit::singleline(&mut app.employee_search)
                .desired_width(200.0)
                .hint_text("Code or name..."),
        );

        ui.add_space(20.0);

        ui.label("Department:");
        egui::ComboBox::from_id_salt("emp_dept_filter")
            .width(180.0)
            .selected_text(
                app.employee_dept_filter
                    .and_then(|id| app.departments.iter().find(|d| d.id == id))
                    .map(|d| d.name.as_str())
                    .unwrap_or("All"),
            )
            .show_ui(ui, |ui| {
                if ui.selectable_label(app.employee_dept_filter.is_none(), "All").clicked() {
                    app.employee_dept_filter = None;
                }
                for dept in &app.departments {
                    if ui
                        .selectable_label(app.employee_dept_filter == Some(dept.id), &dept.name)
                        .clicked()
                    {
                        app.employee_dept_filter = Some(dept.id);
                    }
                }
            });

        ui.add_space(20.0);

        ui.label("Status:");
        if ui
            .selectable_label(app.employee_status_filter.is_none(), "All")
            .clicked()
        {
            app.employee_status_filter = None;
        }
        if ui
            .selectable_label(app.employee_status_filter == Some(true), "Active")
            .clicked()
        {
            app.employee_status_filter = Some(true);
        }
        if ui
            .selectable_label(app.employee_status_filter == Some(false), "Inactive")
            .clicked()
        {
            app.employee_status_filter = Some(false);
        }

        // Clear filters button
        if !app.employee_search.is_empty() || app.employee_dept_filter.is_some() || app.employee_status_filter.is_some()
        {
            ui.add_space(10.0);
            if styled_button(ui, "Clear").clicked() {
                app.employee_search.clear();
                app.employee_dept_filter = None;
                app.employee_status_filter = None;
            }
        }
    });

    ui.add_space(15.0);

    // Table
    show_table(app, ui);

    // Form dialog
    if app.employee_form.is_open {
        show_form_dialog(app, ui.ctx());
    }

    go_back
}

fn show_table(app: &mut App, ui: &mut Ui) {
    // Filter employees
    let filtered: Vec<_> = app
        .employees
        .iter()
        .filter(|e| {
            let search_match = app.employee_search.is_empty()
                || e.employee_code
                    .to_lowercase()
                    .contains(&app.employee_search.to_lowercase())
                || e.full_name.to_lowercase().contains(&app.employee_search.to_lowercase());

            let dept_match = app.employee_dept_filter.is_none() || e.department_id == app.employee_dept_filter;

            let status_match = app.employee_status_filter.is_none() || app.employee_status_filter == Some(e.is_active);

            search_match && dept_match && status_match
        })
        .collect();

    ui.label(format!(
        "Showing {} of {} employees",
        filtered.len(),
        app.employees.len()
    ));

    ui.add_space(10.0);

    ScrollArea::vertical().id_salt("staff_scroll").show(ui, |ui| {
        ui.add_space(4.0);
        egui::Grid::new("employees_grid")
            .num_columns(8)
            .striped(true)
            .min_col_width(60.0)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                // Header
                ui.strong("Code");
                ui.strong("Name");
                ui.strong("Department");
                ui.strong("Device UID");
                ui.strong("Gender");
                ui.strong("Start Date");
                ui.strong("Active");
                ui.strong("Actions");
                ui.end_row();

                // Data rows
                for emp in filtered {
                    ui.label(&emp.employee_code);
                    ui.label(&emp.full_name);

                    let dept_name = emp
                        .department_id
                        .and_then(|id| app.departments.iter().find(|d| d.id == id))
                        .map(|d| d.name.as_str())
                        .unwrap_or("-");
                    ui.label(dept_name);

                    ui.label(emp.scanner_uid.map(|u| u.to_string()).unwrap_or("-".to_string()));

                    ui.label(emp.gender.as_deref().unwrap_or("-"));
                    ui.label(emp.start_date.to_string());
                    ui.label(if emp.is_active { "Yes" } else { "No" });

                    ui.horizontal(|ui| {
                        ui.add_space(8.0);
                        if action_button(ui, PENCIL, "Edit").clicked() {
                            app.employee_form = EmployeeForm::edit(emp);
                        }
                        ui.add_space(4.0);
                        if danger_action_button(ui, TRASH, "Delete").clicked() {
                            app.delete_target = Some(DeleteTarget::Employee(emp.id, emp.full_name.clone()));
                            app.show_delete_confirm = true;
                        }
                    });

                    ui.end_row();
                }
            });
    });
}

fn show_form_dialog(app: &mut App, ctx: &egui::Context) {
    let title = if app.employee_form.is_editing {
        "Edit Employee"
    } else {
        "Add Employee"
    };

    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .default_width(450.0)
        .max_height(500.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.add_space(10.0);

            ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                egui::Grid::new("emp_form_grid")
                    .num_columns(2)
                    .spacing([20.0, 10.0])
                    .show(ui, |ui| {
                        ui.label("Employee Code:");
                        ui.add(egui::TextEdit::singleline(&mut app.employee_form.employee_code).desired_width(200.0));
                        ui.end_row();

                        ui.label("Full Name:");
                        ui.add(egui::TextEdit::singleline(&mut app.employee_form.full_name).desired_width(250.0));
                        ui.end_row();

                        ui.label("Department:");
                        egui::ComboBox::from_id_salt("emp_form_dept")
                            .width(250.0)
                            .selected_text(
                                app.employee_form
                                    .department_id
                                    .and_then(|id| app.departments.iter().find(|d| d.id == id))
                                    .map(|d| d.name.as_str())
                                    .unwrap_or("None"),
                            )
                            .show_ui(ui, |ui| {
                                if ui
                                    .selectable_label(app.employee_form.department_id.is_none(), "None")
                                    .clicked()
                                {
                                    app.employee_form.department_id = None;
                                }
                                for dept in &app.departments {
                                    if ui
                                        .selectable_label(app.employee_form.department_id == Some(dept.id), &dept.name)
                                        .clicked()
                                    {
                                        app.employee_form.department_id = Some(dept.id);
                                    }
                                }
                            });
                        ui.end_row();

                        ui.label("Scanner UID:");
                        ui.add(
                            egui::TextEdit::singleline(&mut app.employee_form.scanner_uid)
                                .desired_width(100.0)
                                .hint_text("Optional"),
                        );
                        ui.end_row();

                        ui.label("Gender:");
                        egui::ComboBox::from_id_salt("emp_form_gender")
                            .width(150.0)
                            .selected_text(app.employee_form.gender.as_deref().unwrap_or("Select..."))
                            .show_ui(ui, |ui| {
                                if ui
                                    .selectable_label(app.employee_form.gender.is_none(), "None")
                                    .clicked()
                                {
                                    app.employee_form.gender = None;
                                }
                                for gender in &["male", "female", "other"] {
                                    if ui
                                        .selectable_label(app.employee_form.gender.as_deref() == Some(*gender), *gender)
                                        .clicked()
                                    {
                                        app.employee_form.gender = Some(gender.to_string());
                                    }
                                }
                            });
                        ui.end_row();

                        ui.label("Start Date:");
                        ui.vertical(|ui| {
                            // Determine if current input is valid
                            let is_valid =
                                app.employee_form.start_date_input.is_empty() || app.employee_form.start_date.is_some();

                            // Red text for invalid input
                            let text_color = if is_valid {
                                ui.visuals().text_color()
                            } else {
                                colors::ERROR
                            };

                            let response = ui.add(
                                egui::TextEdit::singleline(&mut app.employee_form.start_date_input)
                                    .desired_width(120.0)
                                    .hint_text("YYYY-MM-DD")
                                    .text_color(text_color),
                            );

                            // Parse on change - update parsed date if valid
                            if response.changed() {
                                app.employee_form.start_date = parse_flexible_date(&app.employee_form.start_date_input);
                            }

                            // Show format hint (red if invalid)
                            if !is_valid {
                                ui.colored_label(colors::ERROR, "Invalid date format");
                            } else {
                                ui.weak("Format: YYYY-MM-DD");
                            }
                        });
                        ui.end_row();

                        ui.label("Active:");
                        ui.checkbox(&mut app.employee_form.is_active, "");
                        ui.end_row();
                    });
            });

            ui.add_space(15.0);
            ui.separator();
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                if styled_button(ui, "Cancel").clicked() {
                    app.employee_form.reset();
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if primary_button_with_icon(ui, "", "Save").clicked() {
                        save_employee(app);
                    }
                });
            });
        });
}

fn save_employee(app: &mut App) {
    let form = &app.employee_form;

    // Validation
    if form.employee_code.trim().is_empty() {
        app.error_message = Some("Employee code is required".to_string());
        return;
    }
    if form.full_name.trim().is_empty() {
        app.error_message = Some("Full name is required".to_string());
        return;
    }
    let start_date = match form.start_date {
        Some(d) => d,
        None => {
            app.error_message = Some("Start date is required".to_string());
            return;
        }
    };

    let scanner_uid = if form.scanner_uid.is_empty() {
        None
    } else {
        match form.scanner_uid.parse() {
            Ok(uid) => Some(uid),
            Err(_) => {
                app.error_message = Some("Invalid scanner UID (must be a number)".to_string());
                return;
            }
        }
    };

    if form.is_editing {
        let id = form.id.unwrap();
        let data = UpdateEmployee {
            employee_code: Some(form.employee_code.clone()),
            full_name: Some(form.full_name.clone()),
            department_id: Some(form.department_id),
            scanner_uid: Some(scanner_uid),
            gender: Some(form.gender.clone()),
            birth_date: Some(form.birth_date),
            start_date: Some(start_date),
            is_active: Some(form.is_active),
        };
        app.update_employee(id, data);
    } else {
        let data = CreateEmployee {
            employee_code: form.employee_code.clone(),
            full_name: form.full_name.clone(),
            department_id: form.department_id,
            scanner_uid,
            gender: form.gender.clone(),
            birth_date: form.birth_date,
            start_date,
        };
        app.create_employee(data);
    }
}
