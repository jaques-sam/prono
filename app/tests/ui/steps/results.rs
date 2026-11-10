use cucumber::{given, then, when};

use crate::fake_backend::{Health, MY_DEVICE};
use crate::steps::common::SURVEY_ID;
use crate::world::World;

#[given(expr = "{string} already answered from this device")]
fn answered_here(world: &mut World, name: String) {
    world.session().backend.seed(SURVEY_ID, &name, MY_DEVICE, 1, 2030);
}

#[given(expr = "{string} already answered from this device, predicting month {int} of {int}")]
fn answered_here_predicting(world: &mut World, name: String, month: u8, year: u16) {
    world.session().backend.seed(SURVEY_ID, &name, MY_DEVICE, month, year);
}

#[given(expr = "{string} already answered from another device, predicting month {int} of {int}")]
fn answered_elsewhere(world: &mut World, name: String, month: u8, year: u16) {
    // Every participant has a device of their own.
    let device = format!("device-of-{name}");
    world.session().backend.seed(SURVEY_ID, &name, &device, month, year);
}

#[given("the server cannot deliver the results")]
fn results_down(world: &mut World) {
    world.session().backend.results.set(Health::Unreachable);
}

#[given("I choose another survey")]
#[when("I choose another survey")]
fn choose_another(world: &mut World) {
    world.click_button("Choose another survey");
}

#[given("I hover over the latest point of the timeline")]
#[when("I hover over the latest point of the timeline")]
fn hover_latest_point(world: &mut World) {
    let latest = world
        .timeline_points()
        .into_iter()
        .max_by(|a, b| a.x.total_cmp(&b.x))
        .expect("the timeline has points");
    world.harness.hover_at(latest);
    world.settle();
}

#[then(expr = "the timeline shows {int} points")]
fn timeline_points(world: &mut World, count: usize) {
    assert_eq!(world.timeline_points().len(), count);
}

#[then(expr = "each question's timeline shows {int} points")]
fn each_timeline_points(world: &mut World, count: usize) {
    let questions = world.session().state.survey().expect("a survey").questions.len();
    assert_eq!(world.timeline_points().len(), count * questions);
}

#[then(expr = "the tooltip shows the date {string}")]
fn tooltip_shows_date(world: &mut World, date: String) {
    assert_painted(world, &date);
}

#[then(expr = "the tooltip shows the date {string} of {string}")]
fn tooltip_shows_date_of(world: &mut World, date: String, name: String) {
    assert_painted(world, &format!("{date}:\n{name}"));
}

fn assert_painted(world: &World, text: &str) {
    let texts = world.painted_texts();
    assert!(
        texts.iter().any(|painted| painted == text),
        "no {text:?} among {texts:?}"
    );
}

#[then(expr = "I see the thanks to {string}")]
fn sees_thanks(world: &mut World, name: String) {
    let thanks = format!("Thanks {name}, your answers are saved.");
    assert!(world.shows(&thanks), "no \"{thanks}\"");
}

#[then("I see everyone's answers")]
fn sees_all_answers(world: &mut World) {
    assert!(world.shows("All answers"));
}
