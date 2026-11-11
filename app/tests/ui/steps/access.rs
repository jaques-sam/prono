use cucumber::{given, then};

use crate::steps::common::{SURVEY, course};
use crate::world::World;

/// The description of the survey a step names: `Starship` or `course`.
fn description(survey: &str) -> String {
    match survey {
        "Starship" => SURVEY.to_string(),
        "course" => course().description,
        other => panic!("unknown survey \"{other}\""),
    }
}

#[given(expr = "{string} is on the name list of the course survey")]
fn on_name_list(world: &mut World, name: String) {
    world.session().backend.put_on_name_list(&name);
}

#[then("I am asked to fill in my first and last name")]
fn asked_for_name(world: &mut World) {
    assert!(world.shows("Fill in your first and last name to see your surveys."));
}

#[then("I am not asked to fill in my first and last name")]
fn not_asked_for_name(world: &mut World) {
    assert!(!world.shows("Fill in your first and last name"), "the hint is shown");
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
    for description in [SURVEY.to_string(), course().description] {
        assert!(!world.has_button(&description), "\"{description}\" is offered");
    }
}

#[then("I am asked for the pass phrase")]
fn asked_for_pass_phrase(world: &mut World) {
    assert!(world.shows("Pass phrase:"), "there is no pass phrase field");
}

#[then("I am not asked for the pass phrase")]
fn not_asked_for_pass_phrase(world: &mut World) {
    assert!(!world.shows("Pass phrase:"), "there is a pass phrase field");
}
