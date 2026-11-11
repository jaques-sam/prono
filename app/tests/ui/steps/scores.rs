use std::cell::Cell;

use cucumber::{given, then, when};
use prono_api::Answer;

use crate::fake_backend::{QUIZ_DESCRIPTION, QUIZ_ID};
use crate::world::World;

#[given(expr = "I answer the quiz as {string}")]
#[when(expr = "I answer the quiz as {string}")]
fn answer_quiz(world: &mut World, name: String) {
    world.answer_survey_as(&name, QUIZ_DESCRIPTION);
}

#[given(expr = "{string} already got {int} answers of the quiz right from another device")]
fn quiz_taken(world: &mut World, name: String, right: usize) {
    let right = Cell::new(right);
    let device = format!("device-of-{name}");
    world
        .session()
        .backend
        .seed_answers(QUIZ_ID, &name, &device, |question| {
            let correct = question
                .correct_answer
                .clone()
                .expect("a quiz question has a correct answer");
            if right.get() > 0 {
                right.set(right.get() - 1);
                return correct;
            }
            let wrong = question
                .possible_answers
                .iter()
                .map(|option| Answer::Poll(option.clone()))
                .find(|option| !option.is_correct(&correct));
            wrong.expect("a wrong option")
        });
}

#[then(expr = "I see the score {string}")]
fn sees_score(world: &mut World, line: String) {
    assert!(world.shows(&line), "no score \"{line}\"");
}

#[then(expr = "I see the score {string} below {string}")]
fn sees_score_below(world: &mut World, lower: String, higher: String) {
    assert!(
        world.shows_before(&higher, &lower),
        "\"{lower}\" does not follow \"{higher}\""
    );
}

#[then("I see no scores")]
fn sees_no_scores(world: &mut World) {
    assert!(!world.shows("Scores"), "scores are shown");
}
