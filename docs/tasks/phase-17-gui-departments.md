# Phase 17: GUI Departments Panel

## Objective

Implement departments CRUD panel with table and form.

---

## Tasks

### 17.1 Departments Panel

**`src/ui/departments.rs`**

```rust
use crate::app::{App, DeleteTarget, DepartmentForm};
use crate::models::department::{CreateDepartment, UpdateDepartment};
use eframe::egui;

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    ui.heading("Departments");
    ui.separator();
    ui.add_space(10.0);

    // Toolbar
    ui.horizontal(|ui| {
        if ui.button("Add Department").clicked() {
            app.department_form = DepartmentForm {
                is_active: true,
                is_open: true,
                ..Default::default()
            };
        }

        if ui.button("Refresh").clicked() {
            app.load_departments();
        }
    });

    ui.add_space(10.0);

    // Table
    show_table(app, ui);

    // Form dialog
    if app.department_form.is_open {
        show_form_dialog(app, ui.ctx());
    }
}

fn show_table(app: &mut App, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::Grid::new("departments_grid")
            .num_columns(6)
            .striped(true)
            .min_col_width(80.0)
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
                        if ui.small_button("Edit").clicked() {
                            app.department_form = DepartmentForm::edit(dept);
                        }
                        if ui.small_button("Delete").clicked() {
                            app.delete_target = Some(DeleteTarget::Department(
                                dept.id,
                                dept.name.clone(),
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
            egui::Grid::new("dept_form_grid")
                .num_columns(2)
                .spacing([10.0, 10.0])
                .show(ui, |ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut app.department_form.name);
                    ui.end_row();

                    ui.label("Parent:");
                    egui::ComboBox::from_id_salt("dept_parent")
                        .selected_text(
                            app.department_form
                                .parent_id
                                .and_then(|id| {
                                    app.departments.iter().find(|d| d.id == id)
                                })
                                .map(|d| d.name.as_str())
                                .unwrap_or("None"),
                        )
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(
                                    app.department_form.parent_id.is_none(),
                                    "None",
                                )
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
                                    .selectable_label(
                                        app.department_form.parent_id == Some(dept.id),
                                        &dept.name,
                                    )
                                    .clicked()
                                {
                                    app.department_form.parent_id = Some(dept.id);
                                }
                            }
                        });
                    ui.end_row();

                    ui.label("Display Order:");
                    ui.text_edit_singleline(&mut app.department_form.display_order);
                    ui.end_row();

                    ui.label("Active:");
                    ui.checkbox(&mut app.department_form.is_active, "");
                    ui.end_row();
                });

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    app.department_form.reset();
                }

                if ui.button("Save").clicked() {
                    save_department(app);
                }
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
```

### 17.2 App CRUD Methods

Add to `src/app.rs`:

```rust
impl App {
    pub fn load_departments(&mut self) {
        self.is_loading = true;
        self.loading_message = "Loading departments...".to_string();

        let pool = self.pool.clone();
        let tx = self.tx.clone();

        self.rt.spawn(async move {
            match crate::db::department::list_all(&pool).await {
                Ok(depts) => {
                    let _ = tx.send(UiMessage::DepartmentsLoaded(depts));
                }
                Err(e) => {
                    let _ = tx.send(UiMessage::LoadError(e.to_string()));
                }
            }
        });
    }

    pub fn create_department(&mut self, data: CreateDepartment) {
        let pool = self.pool.clone();
        let tx = self.tx.clone();

        self.rt.spawn(async move {
            match crate::db::department::create(&pool, data).await {
                Ok(dept) => {
                    let _ = tx.send(UiMessage::DepartmentSaved(dept));
                }
                Err(e) => {
                    let _ = tx.send(UiMessage::OperationFailed(e.to_string()));
                }
            }
        });
    }

    pub fn update_department(&mut self, id: i32, data: UpdateDepartment) {
        let pool = self.pool.clone();
        let tx = self.tx.clone();

        self.rt.spawn(async move {
            match crate::db::department::update(&pool, id, data).await {
                Ok(Some(dept)) => {
                    let _ = tx.send(UiMessage::DepartmentSaved(dept));
                }
                Ok(None) => {
                    let _ = tx.send(UiMessage::OperationFailed("Department not found".to_string()));
                }
                Err(e) => {
                    let _ = tx.send(UiMessage::OperationFailed(e.to_string()));
                }
            }
        });
    }

    pub fn delete_department(&mut self, id: i32) {
        let pool = self.pool.clone();
        let tx = self.tx.clone();

        self.rt.spawn(async move {
            match crate::db::department::delete(&pool, id).await {
                Ok(true) => {
                    let _ = tx.send(UiMessage::DepartmentDeleted(id));
                }
                Ok(false) => {
                    let _ = tx.send(UiMessage::OperationFailed("Department not found".to_string()));
                }
                Err(e) => {
                    let _ = tx.send(UiMessage::OperationFailed(e.to_string()));
                }
            }
        });
    }

    pub fn confirm_delete(&mut self) {
        if let Some(target) = &self.delete_target {
            match target {
                DeleteTarget::Department(id, _) => {
                    self.delete_department(*id);
                }
                DeleteTarget::Employee(id, _) => {
                    self.delete_employee(*id);
                }
            }
        }
    }
}
```

---

## Deliverables

- [x] Departments table view
- [x] Add/Edit form dialog
- [x] Parent department dropdown
- [x] Form validation
- [x] load_departments() async method
- [x] create_department() async method
- [x] update_department() async method
- [x] delete_department() async method
- [x] Delete confirmation
