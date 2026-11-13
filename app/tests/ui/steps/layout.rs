use cucumber::{given, then, when};
use egui::accesskit::Role;
use egui::{Event, Modifiers, MouseWheelUnit, TouchPhase, pos2, vec2};
use egui_kittest::Node;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use crate::world::World;

/// The width of a text field in a window that is wide enough: egui's default.
const TEXT_EDIT_WIDTH: f32 = 280.0;

/// Tolerance for positions, in points.
const EPSILON: f32 = 1.0;

/// The text field (plain or password) that the screen readers know by `label`.
fn field<'a>(world: &'a World, label: &'a str) -> Node<'a> {
    world
        .harness
        .query_all_by_label(label)
        .find(|node| matches!(node.accesskit_node().role(), Role::TextInput | Role::PasswordInput))
        .unwrap_or_else(|| panic!("no \"{label}\" field"))
}

fn name_fields(world: &World) -> [Node<'_>; 2] {
    [field(world, "First name:"), field(world, "Last name:")]
}

#[given(expr = "the window is {int} points wide")]
fn window_width(world: &mut World, width: u16) {
    world.resize(f32::from(width));
}

#[when(expr = "I scroll right by {int} points")]
fn scroll_right(world: &mut World, points: u16) {
    world.form_left = field(world, "First name:").rect().left();
    let window = world.harness.ctx.content_rect();
    world.harness.hover_at(pos2(window.center().x, window.center().y));
    world.harness.event(Event::MouseWheel {
        unit: MouseWheelUnit::Point,
        delta: vec2(-f32::from(points), 0.0),
        phase: TouchPhase::Move,
        modifiers: Modifiers::NONE,
    });
    // The scrolling is animated.
    for _ in 0..10 {
        world.wait(0.1);
    }
}

#[then("the name fields keep their full width")]
fn name_fields_keep_width(world: &mut World) {
    for name in name_fields(world) {
        let width = name.rect().width();
        assert!(
            width >= TEXT_EDIT_WIDTH - EPSILON,
            "a name field is {width} points wide"
        );
    }
}

#[then(expr = "the form moved left by {int} points")]
fn form_moved_left(world: &mut World, points: u16) {
    let moved = world.form_left - field(world, "First name:").rect().left();
    assert!(
        (moved - f32::from(points)).abs() < EPSILON,
        "the form moved {moved} points"
    );
}
