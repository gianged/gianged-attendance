# Phase 18: GUI Employees Panel

## Objective

Implement employees CRUD panel with search, filter, and form.

---

## Tasks

### 18.1 Employees Panel

**`src/ui/employees.rs`**

```rust
use crate::app::{App, DeleteTarget, EmployeeForm};
use crate::models::employee::{CreateEmployee, UpdateEmployee};
use eframe::egui;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    ui.heading("Employees");
    ui.separator();
    ui.add_space(10.0);

    // Toolbar
    ui.horizontal(|ui| {
        if ui.button("Add Employee").clicked() {
            app.employee_form = EmployeeForm {
                is_active: true,
                start_date: Some(chrono::Local::now().date_naive()),
                is_open: true,
                ..Default::default()
            };
        }

        if ui.button("Refresh").clicked() {
            app.load_employees();
        }

        if ui.button("Export to Excel").clicked() {
            app.export_employees();
        }

        ui.separator();

        // Search
        ui.label("Search:");
        if ui
            .text_edit_singleline(&mut app.employee_search)
            .changed()
        {
            // Search is instant via filtering
        }

        ui.separator();

        // Department filter
        ui.label("Department:");
        egui::ComboBox::from_id_salt("emp_dept_filter")
            .width(150.0)
            .selected_text(
                app.employee_dept_filter
                    .and_then(|id| app.departments.iter().find(|d| d.id == id))
                    .map(|d| d.name.as_str())
                    .unwrap_or("All"),
            )
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(app.employee_dept_filter.is_none(), "All")
                    .clicked()
                {
                    app.employee_dept_filter = None;
                }
                for dept in &app.departments {
                    if ui
                        .selectable_label(
                            app.employee_dept_filter == Some(dept.id),
                            &dept.name,
                        )
                        .clicked()
                    {
                        app.employee_dept_filter = Some(dept.id);
                    }
                }
            });
    });

    ui.add_space(10.0);

    // Table
    show_table(app, ui);

    // Form dialog
    if app.employee_form.is_open {
        show_form_dialog(app, ui.ctx());
    }
}

fn show_table(app: &mut App, ui: &mut egui::Ui) {
    // Filter employees
    let filtered: Vec<_> = app
        .employees
        .iter()
        .filter(|e| {
            let search_match = app.employee_search.is_empty()
                || e.employee_code
                    .to_lowercase()
                    .contains(&app.employee_search.to_lowercase())
                || e.full_name
                    .to_lowercase()
                    .contains(&app.employee_search.to_lowercase());

            let dept_match =
                app.employee_dept_filter.is_none() || e.department_id == app.employee_dept_filter;

            search_match && dept_match
        })
        .collect();

    ui.label(format!("Showing {} of {} employees", filtered.len(), app.employees.len()));

    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::Grid::new("employees_grid")
            .num_columns(8)
            .striped(true)
            .min_col_width(60.0)
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

                    ui.label(
                        emp.device_uid
                            .map(|u| u.to_string())
                            .unwrap_or("-".to_string()),
                    );

                    ui.label(emp.gender.as_deref().unwrap_or("-"));
                    ui.label(emp.start_date.to_string());
                    ui.label(if emp.is_active { "Yes" } else { "No" });

                    ui.horizontal(|ui| {
                        if ui.small_button("Edit").clicked() {
                            app.employee_form = EmployeeForm::edit(emp);
                        }
                        if ui.small_button("Delete").clicked() {
                            app.delete_target = Some(DeleteTarget::Employee(
                                emp.id,
                                emp.full_name.clone(),
                            ));
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
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            egui::Grid::new("emp_form_grid")
                .num_columns(2)
                .spacing([10.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Employee Code:");
                    ui.text_edit_singleline(&mut app.employee_form.employee_code);
                    ui.end_row();

                    ui.label("Full Name:");
                    ui.text_edit_singleline(&mut app.employee_form.full_name);
                    ui.end_row();

                    ui.label("Department:");
                    egui::ComboBox::from_id_salt("emp_form_dept")
                        .selected_text(
                            app.employee_form
                                .department_id
                                .and_then(|id| {
                                    app.departments.iter().find(|d| d.id == id)
                                })
                                .map(|d| d.name.as_str())
                                .unwrap_or("None"),
                        )
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(
                                    app.employee_form.department_id.is_none(),
                                    "None",
                                )
                                .clicked()
                            {
                                app.employee_form.department_id = None;
                            }
                            for dept in &app.departments {
                                if ui
                                    .selectable_label(
                                        app.employee_form.department_id == Some(dept.id),
                                        &dept.name,
                                    )
                                    .clicked()
                                {
                                    app.employee_form.department_id = Some(dept.id);
                                }
                            }
                        });
                    ui.end_row();

                    ui.label("Device UID:");
                    ui.text_edit_singleline(&mut app.employee_form.device_uid);
                    ui.end_row();

                    ui.label("Gender:");
                    egui::ComboBox::from_id_salt("emp_form_gender")
                        .selected_text(
                            app.employee_form
                                .gender
                                .as_deref()
                                .unwrap_or("Select"),
                        )
                        .show_ui(ui, |ui| {
                            for gender in &["male", "female", "other"] {
                                if ui
                                    .selectable_label(
                                        app.employee_form.gender.as_deref() == Some(*gender),
                                        *gender,
                                    )
                                    .clicked()
                                {
                                    app.employee_form.gender = Some(gender.to_string());
                                }
                            }
                        });
                    ui.end_row();

                    ui.label("Start Date:");
                    // Simple date input (YYYY-MM-DD)
                    let mut date_str = app
                        .employee_form
                        .start_date
                        .map(|d| d.to_string())
                        .unwrap_or_default();
                    if ui.text_edit_singleline(&mut date_str).changed() {
                        app.employee_form.start_date =
                            chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").ok();
                    }
                    ui.end_row();

                    ui.label("Active:");
                    ui.checkbox(&mut app.employee_form.is_active, "");
                    ui.end_row();
                });

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    app.employee_form.reset();
                }

                if ui.button("Save").clicked() {
                    save_employee(app);
                }
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

    let device_uid = if form.device_uid.is_empty() {
        None
    } else {
        match form.device_uid.parse() {
            Ok(uid) => Some(uid),
            Err(_) => {
                app.error_message = Some("Invalid device UID".to_string());
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
            device_uid: Some(device_uid),
            gender: Some(form.gender.clone()),
            birth_date: Some(form.birth_date),
            start_date: Some(start_date),
            end_date: Some(form.end_date),
            is_active: Some(form.is_active),
        };
        app.update_employee(id, data);
    } else {
        let data = CreateEmployee {
            employee_code: form.employee_code.clone(),
            full_name: form.full_name.clone(),
            department_id: form.department_id,
            device_uid,
            gender: form.gender.clone(),
            birth_date: form.birth_date,
            start_date,
            end_date: form.end_date,
        };
        app.create_employee(data);
    }
}
```

### 18.2 App CRUD Methods

Add to `src/app.rs`:

```rust
impl App {
    pub fn load_employees(&mut self) {
        self.is_loading = true;
        self.loading_message = "Loading employees...".to_string();

        let pool = self.pool.clone();
        let tx = self.tx.clone();

        self.rt.spawn(async move {
            match crate::db::employee::list_all(&pool).await {
                Ok(emps) => {
                    let _ = tx.send(UiMessage::EmployeesLoaded(emps));
                }
                Err(e) => {
                    let _ = tx.send(UiMessage::LoadError(e.to_string()));
                }
            }
        });
    }

    pub fn create_employee(&mut self, data: CreateEmployee) {
        let pool = self.pool.clone();
        let tx = self.tx.clone();

        self.rt.spawn(async move {
            match crate::db::employee::create(&pool, data).await {
                Ok(emp) => {
                    let _ = tx.send(UiMessage::EmployeeSaved(emp));
                }
                Err(e) => {
                    let _ = tx.send(UiMessage::OperationFailed(e.to_string()));
                }
            }
        });
    }

    pub fn update_employee(&mut self, id: i32, data: UpdateEmployee) {
        let pool = self.pool.clone();
        let tx = self.tx.clone();

        self.rt.spawn(async move {
            match crate::db::employee::update(&pool, id, data).await {
                Ok(Some(emp)) => {
                    let _ = tx.send(UiMessage::EmployeeSaved(emp));
                }
                Ok(None) => {
                    let _ = tx.send(UiMessage::OperationFailed("Employee not found".to_string()));
                }
                Err(e) => {
                    let _ = tx.send(UiMessage::OperationFailed(e.to_string()));
                }
            }
        });
    }

    pub fn delete_employee(&mut self, id: i32) {
        let pool = self.pool.clone();
        let tx = self.tx.clone();

        self.rt.spawn(async move {
            match crate::db::employee::delete(&pool, id).await {
                Ok(true) => {
                    let _ = tx.send(UiMessage::EmployeeDeleted(id));
                }
                Ok(false) => {
                    let _ = tx.send(UiMessage::OperationFailed("Employee not found".to_string()));
                }
                Err(e) => {
                    let _ = tx.send(UiMessage::OperationFailed(e.to_string()));
                }
            }
        });
    }

    pub fn export_employees(&mut self) {
        let filename = crate::export::generate_export_filename("employees");
        let path = std::path::PathBuf::from(&filename);

        match crate::export::export_employees_to_excel(&self.employees, &self.departments, &path) {
            Ok(_) => {
                self.success_message = Some(format!("Exported to: {}", filename));
            }
            Err(e) => {
                self.error_message = Some(format!("Export failed: {}", e));
            }
        }
    }
}
```

---

## Deliverables

- [ ] Employees table view with filtering
- [ ] Search by code/name
- [ ] Filter by department dropdown
- [ ] Add/Edit form dialog
- [ ] Form validation
- [ ] load_employees() async method
- [ ] create_employee() async method
- [ ] update_employee() async method
- [ ] delete_employee() async method
- [ ] export_employees() method
