//! GUI panels and application state.

pub mod components;
pub mod dashboard;
pub mod department_panel;
pub mod main_app;
pub mod setup_wizard;
pub mod staff_panel;
pub mod sync_panel;

pub use main_app::MainApp;
pub use setup_wizard::{SetupApp, SetupWizard};
