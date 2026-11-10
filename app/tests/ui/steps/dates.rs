use cucumber::{given, then, when};

use crate::world::{DatePart, World};

#[given(expr = "I set the {part} of {string} to {int}")]
#[when(expr = "I set the {part} of {string} to {int}")]
fn set_date_part(world: &mut World, part: DatePart, question: String, value: i32) {
    world.type_date(&question, part, value);
}

#[when("I reset my answers")]
#[given("I reset my answers")]
fn reset(world: &mut World) {
    world.click_button("Reset");
}

#[then(expr = "the {part} of {string} shows {int}")]
fn date_part_shows(world: &mut World, part: DatePart, question: String, expected: i32) {
    assert_eq!(world.date_value(&question, part), expected);
}

#[then(expr = "the {part} of {string} no longer shows {int}")]
fn date_part_changed(world: &mut World, part: DatePart, question: String, unexpected: i32) {
    assert_ne!(world.date_value(&question, part), unexpected);
}
