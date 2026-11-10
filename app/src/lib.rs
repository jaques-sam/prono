//! The Prono app: the survey flow ([`state`]), its drawing ([`ui`]) and the
//! REST client ([`client`]). `main.rs` runs them in eframe; the tests in
//! `tests/` drive them headlessly against a fake backend.

#![warn(clippy::all, rust_2018_idioms)]
// The modules are public for `main.rs` and the tests, not as a library API.
#![allow(clippy::must_use_candidate)]

pub mod client;
pub mod state;
pub mod timeline;
pub mod ui;

/// Runs one frame: takes in finished requests, draws the app, and applies
/// what the participant asked for.
pub fn frame(ui: &mut egui::Ui, state: &mut state::State, backend: &impl client::Backend) {
    state.poll(backend);
    if let Some(action) = ui::show(ui, state) {
        state.apply(action, backend);
    }
}
