//! The state shared by the steps of one scenario: the app, running in a
//! headless egui harness against a [`FakeBackend`].

use std::fmt;
use std::str::FromStr;

use egui::accesskit::Role;
use egui::epaint::Shape;
use egui::{Color32, Pos2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, Node};
use prono_app::state::State;
use prono_app::timeline::{OWN_POINT, POINT};

use crate::fake_backend::FakeBackend;

/// What the app is made of, besides its drawing.
pub struct Session {
    pub state: State,
    pub backend: FakeBackend,
}

/// Frames to run after an interaction: enough for a request to finish and its
/// follow-up (fetching the results) to arrive.
const SETTLE_FRAMES: usize = 5;

/// Where the app's clock starts, in seconds.
const START_TIME: f64 = 100.0;

/// Which part of a predicted date: `month` or `year` in a step.
#[derive(Clone, Copy, Debug, cucumber::Parameter)]
#[param(regex = "month|year", name = "part")]
pub enum DatePart {
    Month = 0,
    Year = 1,
}

impl FromStr for DatePart {
    type Err = String;

    fn from_str(part: &str) -> Result<Self, Self::Err> {
        match part {
            "month" => Ok(Self::Month),
            "year" => Ok(Self::Year),
            other => Err(format!("not a date part: {other}")),
        }
    }
}

#[derive(cucumber::World)]
#[world(init = Self::new)]
pub struct World {
    pub harness: Harness<'static, Session>,
    /// The time the app sees, in seconds. It only moves with [`World::wait`],
    /// so typing is instant and pauses are explicit.
    clock: f64,
    /// Whether a scenario entered a pass phrase of its own, to be used by the
    /// next [`World::start_survey`] instead of the right one.
    pub pass_phrase_entered: bool,
}

impl fmt::Debug for World {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("World")
            .field("first_name", &self.session().state.first_name)
            .field("last_name", &self.session().state.last_name)
            .field("error", &self.session().state.error)
            .finish_non_exhaustive()
    }
}

impl World {
    fn new() -> Self {
        let session = Session {
            state: State::default(),
            backend: FakeBackend::new(),
        };
        let harness = Harness::builder().with_size(vec2(700.0, 1000.0)).build_ui_state(
            |ui, session: &mut Session| prono_app::frame(ui, &mut session.state, &session.backend),
            session,
        );
        Self {
            harness,
            clock: START_TIME,
            pass_phrase_entered: false,
        }
    }

    pub fn session(&self) -> &Session {
        self.harness.state()
    }

    /// Lets the app process what just happened, including finished requests.
    pub fn settle(&mut self) {
        self.frames(SETTLE_FRAMES);
    }

    /// Runs `count` frames at the current time.
    pub fn frames(&mut self, count: usize) {
        for _ in 0..count {
            self.harness.input_mut().time = Some(self.clock);
            self.harness.step();
        }
    }

    /// Lets `seconds` pass, with the app showing frames meanwhile.
    pub fn wait(&mut self, seconds: f64) {
        self.clock += seconds;
        self.settle();
    }

    pub fn click_button(&mut self, label: &str) {
        self.harness.get_by_role_and_label(Role::Button, label).click();
        self.settle();
    }

    pub fn has_button(&self, label: &str) -> bool {
        self.harness.query_by_role_and_label(Role::Button, label).is_some()
    }

    /// Whether some label or heading contains `text`.
    pub fn shows(&self, text: &str) -> bool {
        self.harness.query_all_by_label_contains(text).next().is_some()
    }

    /// Whether `first` and `second` are both shown, `first` above `second`.
    pub fn shows_before(&self, first: &str, second: &str) -> bool {
        let top = |text: &str| self.harness.query_by_label_contains(text).map(|node| node.rect().top());
        top(first).zip(top(second)).is_some_and(|(a, b)| a < b)
    }

    /// The month or year input of the question `question`.
    pub fn date_input(&self, question: &str, part: DatePart) -> Node<'_> {
        let index = self
            .session()
            .state
            .survey()
            .and_then(|survey| survey.questions.iter().position(|q| q.text == question))
            .unwrap_or_else(|| panic!("no question \"{question}\" in the survey"));
        // Each question has a month input followed by a year input.
        self.harness
            .query_all_by_role(Role::SpinButton)
            .nth(index * 2 + part as usize)
            .unwrap_or_else(|| panic!("no {part:?} input for \"{question}\""))
    }

    /// The number a date input shows.
    pub fn date_value(&self, question: &str, part: DatePart) -> i32 {
        let shown = self
            .date_input(question, part)
            .accesskit_node()
            .numeric_value()
            .expect("a numeric date input");
        #[allow(clippy::cast_possible_truncation)]
        let value = shown as i32;
        value
    }

    /// Types `value` into a date input and confirms it.
    pub fn type_date(&mut self, question: &str, part: DatePart, value: i32) {
        self.date_input(question, part).click();
        self.settle();
        self.harness.event(egui::Event::Text(value.to_string()));
        self.frames(2);
        self.harness.key_press(egui::Key::Enter);
        self.settle();
    }

    /// Where the timelines currently draw their points.
    pub fn timeline_points(&self) -> Vec<Pos2> {
        self.points_filled(&[POINT, OWN_POINT])
    }

    /// Where the timelines currently draw the points of the participant's own
    /// answers.
    pub fn own_timeline_points(&self) -> Vec<Pos2> {
        self.points_filled(&[OWN_POINT])
    }

    fn points_filled(&self, fills: &[Color32]) -> Vec<Pos2> {
        let mut points = Vec::new();
        for clipped in &self.harness.output().shapes {
            visit(&clipped.shape, &mut |shape| {
                if let Shape::Circle(circle) = shape
                    && fills.contains(&circle.fill)
                {
                    points.push(circle.center);
                }
            });
        }
        points
    }

    /// Everything the app drew as text, for what is painted rather than shown
    /// as widgets (the timeline's tooltips).
    pub fn painted_texts(&self) -> Vec<String> {
        let mut texts = Vec::new();
        for clipped in &self.harness.output().shapes {
            visit(&clipped.shape, &mut |shape| {
                if let Shape::Text(text) = shape {
                    texts.push(text.galley.text().to_string());
                }
            });
        }
        texts
    }
}

fn visit(shape: &Shape, on_shape: &mut impl FnMut(&Shape)) {
    on_shape(shape);
    if let Shape::Vec(shapes) = shape {
        for shape in shapes {
            visit(shape, on_shape);
        }
    }
}
