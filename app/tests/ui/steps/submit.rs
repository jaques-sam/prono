use cucumber::{given, then, when};
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::fake_backend::{FEEDBACK_TITLE, PASS_PHRASE};
use crate::world::World;

#[given("the server is unreachable")]
fn server_unreachable(world: &mut World) {
    world.session().backend.go_offline();
}

#[given("the server is back online")]
fn server_online(world: &mut World) {
    world.session().backend.go_online();
}

#[given("the server does not respond")]
fn server_hangs(world: &mut World) {
    world.session().backend.hang();
}

#[given(expr = "I answer the feedback survey as {string}")]
#[when(expr = "I answer the feedback survey as {string}")]
fn answer_feedback(world: &mut World, name: String) {
    world.answer_survey_as(&name, FEEDBACK_TITLE, PASS_PHRASE);
}

#[given("I submit my answers")]
#[when("I submit my answers")]
fn submit(world: &mut World) {
    world.click_button("Submit");
}

#[then(expr = "the server received the answers of {string}")]
fn server_received(world: &mut World, name: String) {
    assert_eq!(world.session().backend.submitted_by.borrow().last(), Some(&name));
}

#[then(expr = "the server received {int} submissions")]
fn server_received_count(world: &mut World, count: usize) {
    assert_eq!(world.session().backend.submitted_by.borrow().len(), count);
}

#[then(expr = "the {string} button is enabled")]
fn button_enabled(world: &mut World, label: String) {
    let button = world
        .harness
        .get_by_role_and_label(egui::accesskit::Role::Button, &label);
    assert!(!button.accesskit_node().is_disabled(), "\"{label}\" is disabled");
}

#[then(expr = "the {string} button is disabled")]
fn button_disabled(world: &mut World, label: String) {
    let button = world
        .harness
        .get_by_role_and_label(egui::accesskit::Role::Button, &label);
    assert!(button.accesskit_node().is_disabled(), "\"{label}\" is enabled");
}
