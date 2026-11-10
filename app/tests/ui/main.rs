//! Component tests of the app's UI: Gherkin scenarios (`tests/features/`)
//! driving the egui UI headlessly with `egui_kittest`, against an in-memory
//! fake of the backend.
//!
//! Run them with `cargo test -p prono-app --test ui`.

// Step functions take their parameters by value, as the cucumber macros hand them over.
#![allow(clippy::needless_pass_by_value)]

mod fake_backend;
mod steps;
mod world;

use cucumber::World as _;
use world::World;

fn main() {
    futures::executor::block_on(
        World::cucumber()
            .fail_on_skipped()
            .run_and_exit(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/features")),
    );
}
