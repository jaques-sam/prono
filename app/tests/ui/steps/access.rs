use cucumber::{given, then, when};
use egui::accesskit::Role;
use egui_kittest::kittest::Queryable as _;

use crate::fake_backend::{PASS_PHRASE, QUIZ_DESCRIPTION};
use crate::steps::common::{SURVEY, SURVEY_ID, course};
use crate::world::World;

/// The description of the survey a step names: `Starship`, `course` or `quiz`.
fn description(survey: &str) -> String {
    match survey {
        "Starship" => SURVEY.to_string(),
        "course" => course().description,
        "quiz" => QUIZ_DESCRIPTION.to_string(),
        other => panic!("unknown survey \"{other}\""),
    }
}

/// The number of pass phrase fields on screen.
fn pass_phrase_fields(world: &World) -> usize {
    world
        .harness
        .query_all_by_role_and_label(Role::PasswordInput, "Pass phrase:")
        .count()
}

#[given("the pass phrase of the Starship survey was changed on the server")]
fn pass_phrase_changed(world: &mut World) {
    world
        .session()
        .backend
        .change_pass_phrase(SURVEY_ID, &format!("not {PASS_PHRASE} any more"));
}

#[when(expr = "I click the {word} survey")]
fn click_survey(world: &mut World, survey: String) {
    world.click_button(&description(&survey));
}

#[then("I am asked for my pass phrase")]
fn asked_for_pass_phrase(world: &mut World) {
    assert_eq!(pass_phrase_fields(world), 1, "there is no pass phrase field");
    assert!(world.shows("Enter your pass phrase and press Submit to see your surveys."));
}

#[then("I am not asked for my pass phrase")]
fn not_asked_for_pass_phrase(world: &mut World) {
    assert!(!world.shows("Enter your pass phrase"), "the hint is shown");
}

#[then("there is exactly one pass phrase field")]
fn one_pass_phrase_field(world: &mut World) {
    assert_eq!(pass_phrase_fields(world), 1);
}

#[then("there is no pass phrase field")]
fn no_pass_phrase_field(world: &mut World) {
    assert_eq!(pass_phrase_fields(world), 0, "there is a pass phrase field");
}

#[then(expr = "I am offered the {word} survey")]
fn offered(world: &mut World, survey: String) {
    assert!(
        world.has_button(&description(&survey)),
        "the {survey} survey is not offered"
    );
}

#[then(expr = "I am not offered the {word} survey")]
fn not_offered(world: &mut World, survey: String) {
    assert!(
        !world.has_button(&description(&survey)),
        "the {survey} survey is offered"
    );
}

#[then("I am offered no surveys")]
fn offered_nothing(world: &mut World) {
    for description in [SURVEY.to_string(), course().description, QUIZ_DESCRIPTION.to_string()] {
        assert!(!world.has_button(&description), "\"{description}\" is offered");
    }
}

#[then("the server was not asked for any surveys")]
fn asked_for_no_surveys(world: &mut World) {
    assert_eq!(
        *world.session().backend.survey_requests.borrow(),
        Vec::<Option<String>>::new()
    );
}

#[then(expr = "the server was asked for surveys {int} time(s)")]
fn asked_for_surveys(world: &mut World, times: usize) {
    assert_eq!(world.session().backend.survey_requests.borrow().len(), times);
}
