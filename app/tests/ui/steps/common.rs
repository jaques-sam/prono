use cucumber::{given, then, when};
use egui::accesskit::Role;
use egui_kittest::kittest::Queryable as _;

use crate::world::World;

/// The survey the scenarios use.
pub const SURVEY: &str = "SpaceX Starship achievements survey";
pub const SURVEY_ID: u64 = 1;

impl World {
    /// Replaces whatever is in the name field with `name`.
    pub fn enter_name(&mut self, name: &str) {
        self.harness.get_by_role(Role::TextInput).click();
        self.settle();
        self.harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
        self.frames(1);
        if !name.is_empty() {
            self.harness.event(egui::Event::Text(name.to_string()));
        }
        self.settle();
    }
}

#[given("the Prono app is open")]
fn app_is_open(world: &mut World) {
    world.settle();
}

#[given(expr = "I am answering the survey as {string}")]
fn answering_as(world: &mut World, name: String) {
    world.enter_name(&name);
    world.click_button(SURVEY);
    assert!(world.has_button("Submit"), "the survey was not started");
}

#[when(expr = "I enter the name {string}")]
#[given(expr = "I entered the name {string}")]
fn enter_name(world: &mut World, name: String) {
    world.enter_name(&name);
}

#[when(expr = "I enter a name of {int} characters")]
fn enter_long_name(world: &mut World, length: usize) {
    world.enter_name(&"a".repeat(length));
}

#[given("I start the survey")]
#[when("I start the survey")]
fn start_survey(world: &mut World) {
    world.click_button(SURVEY);
}

#[given("I go back to change my name")]
#[when("I go back to change my name")]
fn change_name(world: &mut World) {
    world.click_button("Change name");
}

#[then(expr = "the name field shows {string}")]
fn name_field_shows(world: &mut World, name: String) {
    assert_eq!(world.session().state.user_name, name);
}

#[given("I dismiss the error")]
#[when("I dismiss the error")]
fn dismiss_error(world: &mut World) {
    world.click_button("✖");
}

#[then(expr = "I see the error {string}")]
fn sees_error(world: &mut World, error: String) {
    assert!(world.shows(&error), "no error \"{error}\" is shown");
}

#[then("I see no error")]
fn sees_no_error(world: &mut World) {
    assert!(!world.has_button("✖"), "an error is shown");
}

#[then("I am on the start screen")]
fn on_start_screen(world: &mut World) {
    assert!(world.shows("Choose a survey:"));
    assert!(world.has_button(SURVEY));
}

#[then("I am answering the survey")]
fn is_answering(world: &mut World) {
    assert!(world.shows(SURVEY), "the survey heading is missing");
    assert!(world.has_button("Submit"), "there is no Submit button");
}

#[then(expr = "I see the question {string}")]
fn sees_question(world: &mut World, question: String) {
    assert!(world.shows(&question), "the question \"{question}\" is missing");
}

#[when(expr = "I wait {float} seconds")]
#[given(expr = "I wait {float} seconds")]
fn wait(world: &mut World, seconds: f64) {
    world.wait(seconds);
}

#[then(expr = "I see the warning that {string} already filled in a survey")]
fn sees_name_warning(world: &mut World, name: String) {
    let warning = format!("\"{name}\" already filled in a survey");
    assert!(world.shows(&warning), "no warning \"{warning}\"");
}

#[then("I see no name warning")]
fn sees_no_name_warning(world: &mut World) {
    assert!(!world.shows("already filled in a survey"), "a name warning is shown");
}

#[then("the server was not asked about any name")]
fn asked_no_name(world: &mut World) {
    assert_eq!(*world.session().backend.name_checks.borrow(), Vec::<String>::new());
}

#[then(expr = "the server was asked only about {string}")]
fn asked_only_about(world: &mut World, name: String) {
    assert_eq!(*world.session().backend.name_checks.borrow(), [name]);
}
