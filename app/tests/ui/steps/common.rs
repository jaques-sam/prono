use cucumber::{given, then, when};
use egui::accesskit::Role;
use egui_kittest::kittest::Queryable as _;

use prono_api::Survey;
use prono_app::state::NAME_CHECK_DELAY;

use crate::world::World;

/// The survey the scenarios use.
pub const SURVEY: &str = "SpaceX Starship achievements survey";
pub const SURVEY_ID: u64 = 1;
/// The survey with poll questions.
pub const COURSE_ID: u64 = 2;

/// The survey with poll questions.
pub fn course() -> &'static Survey {
    prono_api::survey(COURSE_ID).expect("the course survey exists")
}

impl World {
    /// Replaces whatever is in the text field `label` with `text`.
    fn type_into(&mut self, label: &str, text: &str) {
        self.harness.get_by_role_and_label(Role::TextInput, label).click();
        self.settle();
        self.harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
        self.frames(1);
        if !text.is_empty() {
            self.harness.event(egui::Event::Text(text.to_string()));
        }
        self.settle();
    }

    /// Replaces whatever is in the name fields. The server is not asked
    /// about the name before the typing paused, see [`World::wait`].
    pub fn enter_name(&mut self, first: &str, last: &str) {
        self.type_into("First name:", first);
        self.type_into("Last name:", last);
    }

    /// Starts the survey, after the typing of the name paused long enough for
    /// the server to tell which surveys the name is offered.
    pub fn start_survey(&mut self, description: &str) {
        self.wait(NAME_CHECK_DELAY + 0.1);
        self.click_button(description);
    }

    /// Enters `name` ("First Last") and starts the survey `description`.
    pub fn answer_survey_as(&mut self, name: &str, description: &str) {
        self.enter_name_parts(name);
        self.start_survey(description);
        assert!(self.has_button("Submit"), "the survey was not started");
    }

    /// Enters `name` ("First Last"; without a space it is a first name only).
    pub fn enter_name_parts(&mut self, name: &str) {
        let (first, last) = name.split_once(' ').unwrap_or((name, ""));
        self.enter_name(first, last);
    }
}

#[given("the Prono app is open")]
fn app_is_open(world: &mut World) {
    world.settle();
}

#[given(expr = "I am answering the survey as {string}")]
fn answering_as(world: &mut World, name: String) {
    world.answer_survey_as(&name, SURVEY);
}

#[when(expr = "I enter the name {string}")]
#[given(expr = "I entered the name {string}")]
fn enter_name(world: &mut World, name: String) {
    world.enter_name_parts(&name);
}

#[when(expr = "I enter the first name {string} and the last name {string}")]
#[given(expr = "I entered the first name {string} and the last name {string}")]
fn enter_name_parts(world: &mut World, first: String, last: String) {
    world.enter_name(&first, &last);
}

#[when(expr = "I enter a first name of {int} characters and the last name {string}")]
fn enter_long_name(world: &mut World, length: usize, last: String) {
    world.enter_name(&"a".repeat(length), &last);
}

#[given("I start the survey")]
#[when("I start the survey")]
fn start_survey(world: &mut World) {
    world.start_survey(SURVEY);
}

#[given("I go back to change my name")]
#[when("I go back to change my name")]
fn change_name(world: &mut World) {
    world.click_button("Change name");
}

#[then(expr = "the name fields show {string} and {string}")]
fn name_fields_show(world: &mut World, first: String, last: String) {
    let state = &world.session().state;
    assert_eq!(
        (state.first_name.as_str(), state.last_name.as_str()),
        (first.as_str(), last.as_str())
    );
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
