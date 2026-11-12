use cucumber::{given, then, when};
use egui::accesskit::Role;
use egui_kittest::kittest::Queryable as _;

use prono_api::Survey;
use prono_app::state::NAME_CHECK_DELAY;

use crate::fake_backend::PASS_PHRASE;
use crate::world::World;
use prono_app::state::Phase;

/// The label of the pass phrase field.
const PASS_PHRASE_LABEL: &str = "Pass phrase:";

/// The survey the scenarios use.
pub const SURVEY: &str = "SpaceX Starship";
pub const SURVEY_ID: u64 = 1;
/// The survey with poll questions.
pub fn course() -> Survey {
    prono_api::samples::course()
}

impl World {
    /// Replaces whatever is in the text field `label` with `text`.
    fn type_into(&mut self, label: &str, text: &str) {
        self.type_into_role(Role::TextInput, label, text);
    }

    fn type_into_role(&mut self, role: Role, label: &str, text: &str) {
        self.harness.get_by_role_and_label(role, label).click();
        self.replace_focused_text(text);
    }

    /// Replaces the text of the field that was just clicked.
    fn replace_focused_text(&mut self, text: &str) {
        self.settle();
        self.harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
        self.frames(1);
        if text.is_empty() {
            self.harness.key_press(egui::Key::Backspace);
        } else {
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

    /// Starts the survey `description`, which `pass_phrase` opens. The
    /// surveys are offered once the pass phrase was submitted: unless a
    /// scenario did that already, it is submitted now.
    pub fn start_survey(&mut self, description: &str, pass_phrase: &str) {
        self.wait(NAME_CHECK_DELAY + 0.1);
        if !self.has_button(description) {
            self.submit_pass_phrase(pass_phrase);
        }
        self.click_button(description);
    }

    /// Replaces whatever is in the pass phrase field with `pass_phrase`.
    pub fn enter_pass_phrase(&mut self, pass_phrase: &str) {
        self.type_into_role(Role::PasswordInput, PASS_PHRASE_LABEL, pass_phrase);
    }

    /// Enters `pass_phrase` and presses Submit.
    pub fn submit_pass_phrase(&mut self, pass_phrase: &str) {
        self.enter_pass_phrase(pass_phrase);
        self.click_button("Submit");
    }

    /// Enters `name` ("First Last"), starts the survey `description` that
    /// `pass_phrase` opens, and checks its questions show.
    pub fn answer_survey_as(&mut self, name: &str, description: &str, pass_phrase: &str) {
        self.enter_name_parts(name);
        self.start_survey(description, pass_phrase);
        assert!(self.is_answering(), "the survey was not started");
    }

    /// Whether the questions of a survey are shown.
    pub fn is_answering(&self) -> bool {
        matches!(self.session().state.phase, Phase::Answering { .. })
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
    world.answer_survey_as(&name, SURVEY, PASS_PHRASE);
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
    world.start_survey(SURVEY, PASS_PHRASE);
}

#[given(expr = "I entered the pass phrase {string}")]
#[when(expr = "I enter the pass phrase {string}")]
fn enter_pass_phrase(world: &mut World, pass_phrase: String) {
    world.enter_pass_phrase(&pass_phrase);
}

#[given("I submitted the pass phrase")]
#[when("I submit the pass phrase")]
fn submit_pass_phrase(world: &mut World) {
    world.click_button("Submit");
}

#[given("I go back to change my name")]
#[when("I go back to change my name")]
fn change_name(world: &mut World) {
    world.click_button("Other survey");
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
    assert!(world.is_answering(), "the questions are not shown");
    assert!(world.shows(SURVEY), "the survey heading is missing");
}

#[then(expr = "there is no {string} button")]
fn has_no_button(world: &mut World, label: String) {
    assert!(!world.has_button(&label), "there is a \"{label}\" button");
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

#[then(expr = "the server was asked about {string} and then about {string}")]
fn asked_about_twice(world: &mut World, first: String, second: String) {
    assert_eq!(*world.session().backend.name_checks.borrow(), [first, second]);
}
