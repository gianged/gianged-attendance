//! GUI panels and application state.

pub mod app;
pub mod components;
pub mod dashboard;
pub mod department_panel;
pub mod reports_panel;
pub mod settings_panel;
pub mod setup_wizard;
pub mod staff_panel;
pub mod sync_panel;

pub use app::App;
pub use setup_wizard::{SetupApp, SetupWizard};
