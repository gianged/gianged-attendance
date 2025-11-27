//! Sync panel for device synchronization.

use chrono::{DateTime, Local};
use eframe::egui::{self, ProgressBar, Ui};

use super::components::{back_button, colors, panel_header};
use super::main_app::SyncState;

/// Action returned from the sync panel.
pub enum Action {
    None,
    GoBack,
    StartSync,
}

/// Show the sync panel.
///
/// Returns the action to take.
pub fn show(ui: &mut Ui, sync_state: &SyncState, last_sync_time: &Option<DateTime<Local>>) -> Action {
    let mut action = Action::None;

    if back_button(ui) {
        action = Action::GoBack;
    }

    panel_header(ui, "Device Sync");

    // Last sync time
    ui.horizontal(|ui| {
        ui.label("Last sync:");
        if let Some(time) = last_sync_time {
            ui.label(time.format("%Y-%m-%d %H:%M:%S").to_string());
        } else {
            ui.label("Never");
        }
    });

    ui.add_space(20.0);

    // Status indicator
    match sync_state {
        SyncState::Idle => {
            ui.colored_label(colors::NEUTRAL, "Status: Idle");
        }
        SyncState::InProgress { progress, message } => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(format!("Syncing: {}", message));
            });
            ui.add_space(10.0);
            ui.add(ProgressBar::new(*progress).show_percentage());
        }
        SyncState::Completed { records_synced } => {
            ui.colored_label(colors::SUCCESS, format!("Completed: {} records synced", records_synced));
        }
        SyncState::Error(err) => {
            ui.colored_label(colors::ERROR, format!("Error: {}", err));
        }
    }

    ui.add_space(30.0);

    // Sync button
    let can_sync = matches!(
        sync_state,
        SyncState::Idle | SyncState::Completed { .. } | SyncState::Error(_)
    );

    if ui.add_enabled(can_sync, egui::Button::new("Sync Now")).clicked() {
        action = Action::StartSync;
    }

    action
}
