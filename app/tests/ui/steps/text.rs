use cucumber::{given, then, when};
use egui::accesskit::Role;
use egui_kittest::kittest::Queryable as _;

use crate::fake_backend::{ESSAY_TITLE, PASS_PHRASE};
use crate::world::World;

/// The height of the input field with `role`, the only one on screen.
fn field_height(world: &World, role: Role) -> f32 {
    let mut fields = world.harness.query_all_by_role(role);
    let field = fields.next().unwrap_or_else(|| panic!("no {role:?} field"));
    assert!(fields.next().is_none(), "more than one {role:?} field");
    field.rect().height()
}

#[given(expr = "I answer the essay survey as {string}")]
#[when(expr = "I answer the essay survey as {string}")]
fn answer_essay(world: &mut World, name: String) {
    world.answer_survey_as(&name, ESSAY_TITLE, PASS_PHRASE);
}

#[when(expr = "I type {int} words into the multiline answer")]
fn type_words(world: &mut World, words: usize) {
    world.harness.get_by_role(Role::MultilineTextInput).click();
    world.settle();
    world.harness.event(egui::Event::Text("word ".repeat(words)));
    world.settle();
}

#[then("the multiline answer field is taller than the single-line one")]
fn multiline_is_taller(world: &mut World) {
    let single = field_height(world, Role::TextInput);
    let multi = field_height(world, Role::MultilineTextInput);
    assert!(
        multi > single * 2.0,
        "multiline {multi} is not much taller than {single}"
    );
}

#[given("I note the height of the multiline answer field")]
#[when("I note the height of the multiline answer field")]
fn note_height(world: &mut World) {
    world.field_height = field_height(world, Role::MultilineTextInput);
}

#[then("the multiline answer field is taller than noted")]
fn taller_than_noted(world: &mut World) {
    let height = field_height(world, Role::MultilineTextInput);
    assert!(
        height > world.field_height,
        "{height} is not taller than {}",
        world.field_height
    );
}
