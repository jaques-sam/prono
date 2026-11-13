//! Draws the [`State`] and reports what the participant clicked. All rules
//! live in `prono_api` and [`State`]; this module only renders.

use egui::{FontId, RichText, TextEdit};
use prono_api::{Answer, AnswerKind, PREDICTION_YEARS, Question, Results, Score, Survey};

use crate::state::{Action, CAN_RESET, Phase, State};
use crate::{background, poll, timeline};

pub fn show(ui: &mut egui::Ui, state: &mut State) -> Option<Action> {
    egui::Panel::top("top_panel").show(ui, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            if !cfg!(target_arch = "wasm32") {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        ui.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.add_space(16.0);
            }
            egui::widgets::global_theme_preference_buttons(ui);
        });
    });

    egui::Panel::bottom("footer").show(ui, |ui| {
        footer(ui);
        egui::warn_if_debug_build(ui);
    });

    egui::CentralPanel::default()
        .frame(egui::Frame::NONE)
        .show(ui, |ui| {
            background::paint(ui.painter(), ui.max_rect());
            card(ui, |ui| {
                ui.heading("Prono");
                let error_action = state.error.as_deref().and_then(|error| error_banner(ui, error));

                let busy = state.is_busy();
                let warning = state.name_warning();
                let user = prono_api::full_name(&state.first_name, &state.last_name);
                let phase_action = match &mut state.phase {
                    Phase::Answering { survey, answers } => answering(ui, survey, answers, busy),
                    Phase::Done { survey } => done(ui, survey, &user, state.results.as_ref(), state.scores.as_deref()),
                    Phase::Start => start(ui, state, warning.as_deref()),
                };
                if !matches!(state.phase, Phase::Answering { .. }) {
                    poll::forget_other(ui.ctx());
                }
                error_action.or(phase_action)
            })
        })
        .inner
}

/// The widest the card gets; on a wider window it stays centered.
const CARD_MAX_WIDTH: f32 = 640.0;
/// The narrowest the card gets; a narrower window scrolls sideways instead of
/// squashing the content.
const CARD_MIN_WIDTH: f32 = 480.0;
/// The space inside the card, between its border and its elements.
const CARD_PADDING: i8 = 20;
/// The space between the card and the edge of the window.
const CARD_MARGIN: i8 = 12;
/// How much of the background shines through the card: `bg-pine/20`.
const CARD_TINT_ALPHA: u8 = 51;
/// `border-pine/50`.
const CARD_BORDER_ALPHA: u8 = 128;
const CARD_CORNER_RADIUS: u8 = 16;
/// The space between the elements in the card.
const CARD_ITEM_SPACING: egui::Vec2 = egui::vec2(12.0, 12.0);

/// Draws `content` in a rounded square, centered in the available space. When
/// the window is smaller than the card, the window scrolls, vertically and
/// horizontally.
fn card<R>(ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui) -> R) -> R {
    // Inside a horizontal scroll area the available width is unbounded.
    let window_width = ui.available_width();
    egui::ScrollArea::both()
        .auto_shrink(false)
        .show(ui, |ui| {
            ui.set_width(window_width.max(CARD_MIN_WIDTH));
            centered_card(ui, content)
        })
        .inner
}

fn centered_card<R>(ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let width = ui.available_width().min(CARD_MAX_WIDTH);
    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(width, ui.available_height()),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                let visuals = ui.visuals();
                // The light theme keeps its solid panel color, so the text stays readable.
                let fill = if visuals.dark_mode {
                    background::pine(CARD_TINT_ALPHA)
                } else {
                    visuals.panel_fill
                };
                egui::Frame::group(ui.style())
                    .fill(fill)
                    .stroke(egui::Stroke::new(1.0, background::pine(CARD_BORDER_ALPHA)))
                    .outer_margin(egui::Margin::same(CARD_MARGIN))
                    .inner_margin(egui::Margin::same(CARD_PADDING))
                    .corner_radius(egui::CornerRadius::same(CARD_CORNER_RADIUS))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.spacing_mut().item_spacing = CARD_ITEM_SPACING;
                        content(ui)
                    })
                    .inner
            },
        )
        .inner
    })
    .inner
}

fn error_banner(ui: &mut egui::Ui, error: &str) -> Option<Action> {
    let color = ui.visuals().error_fg_color;
    egui::Frame::new()
        .fill(color.gamma_multiply(0.15))
        .inner_margin(egui::Margin::same(6))
        .corner_radius(egui::CornerRadius::same(4))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.colored_label(color, error);
                ui.small_button("✖").clicked().then_some(Action::DismissError)
            })
            .inner
        })
        .inner
}

/// The width of a "Submit" button next to a text field, so that both together
/// are as wide as a text field alone.
const SUBMIT_WIDTH: f32 = 72.0;

/// A row of the form: a `label` the screen readers know the single-line text
/// field by. Call it in a grid, so the fields line up.
fn labelled_text_edit(ui: &mut egui::Ui, label: &str, hint: &str, text: &mut String, password: bool) -> egui::Response {
    let label = ui.label(label);
    ui.add(TextEdit::singleline(text).hint_text(hint).password(password))
        .labelled_by(label.id)
}

fn start(ui: &mut egui::Ui, state: &mut State, warning: Option<&str>) -> Option<Action> {
    // The one pass phrase field: what it opens is only asked from the backend
    // on a click (or Enter), never while typing.
    let busy = state.is_busy();
    let mut action = None;
    egui::Grid::new("start").num_columns(2).show(ui, |ui| {
        labelled_text_edit(ui, "First name:", "Your first name", &mut state.first_name, false);
        ui.end_row();
        labelled_text_edit(ui, "Last name:", "Your last name", &mut state.last_name, false);
        ui.end_row();
        if let Some(warning) = warning {
            ui.label("");
            ui.colored_label(ui.visuals().warn_fg_color, warning);
            ui.end_row();
        }

        let label = ui.label("Pass phrase:");
        ui.horizontal(|ui| {
            // The field and the button are as wide as the other fields.
            let width = ui.spacing().text_edit_width - SUBMIT_WIDTH - ui.spacing().item_spacing.x;
            let field = ui
                .add(
                    TextEdit::singleline(&mut state.pass_phrase)
                        .hint_text("The pass phrase you were given")
                        .password(true)
                        .desired_width(width),
                )
                .labelled_by(label.id);
            let entered = field.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
            let button = egui::Button::new("Submit").min_size(egui::vec2(SUBMIT_WIDTH, 0.0));
            let clicked = ui.add_enabled(!busy, button).clicked();
            if busy {
                ui.spinner();
            }
            action = (clicked || entered).then_some(Action::SubmitPassPhrase);
        });
        ui.end_row();
    });
    ui.add_space(8.0);

    match state.open_surveys() {
        None => {
            ui.label("Enter your pass phrase and press Submit to see your surveys.");
        }
        Some(surveys) => {
            ui.label("Choose a survey:");
            for survey in surveys {
                if ui.button(&survey.title).clicked() {
                    action = Some(Action::StartSurvey(survey.id));
                }
            }
        }
    }
    action
}

fn answering(ui: &mut egui::Ui, survey: &Survey, answers: &mut [Answer], busy: bool) -> Option<Action> {
    survey_heading(ui, survey);

    egui::Grid::new("questions")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            for (question, answer) in survey.questions.iter().zip(answers.iter_mut()) {
                if question.answer_optional {
                    ui.label(format!("{} (optional)", question.text));
                } else {
                    ui.label(&question.text);
                }
                answer_input(ui, question, answer);
                ui.end_row();
            }
        });

    let action = ui
        .horizontal(|ui| {
            let mut action = None;
            if CAN_RESET && ui.add_enabled(!busy, egui::Button::new("Reset")).clicked() {
                action = Some(Action::ResetAnswers);
            }
            if ui.add_enabled(!busy, egui::Button::new("Submit")).clicked() {
                action = Some(Action::Submit);
            }
            if ui.add_enabled(!busy, egui::Button::new("Other survey")).clicked() {
                action = Some(Action::OtherSurvey);
            }
            if busy {
                ui.spinner();
            }
            action
        })
        .inner;
    if matches!(action, Some(Action::ResetAnswers)) {
        poll::forget_other(ui.ctx());
    }

    if survey
        .questions
        .iter()
        .any(|question| question.kind == AnswerKind::PredictionDate)
    {
        ui.separator();
        ui.heading("Timeline of your predictions");
        let dates = timeline::extract_and_sort_dates(answers.iter().map(|answer| (None, answer)));
        timeline::draw(ui, &dates, None);
    }

    action
}

fn answer_input(ui: &mut egui::Ui, question: &Question, answer: &mut Answer) {
    match answer {
        Answer::Text(text) => {
            ui.add(TextEdit::singleline(text).hint_text("your answer here"));
        }
        Answer::PredictionDate { month, year, .. } => {
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(month).range(1..=12).prefix("month "));
                ui.add(egui::DragValue::new(year).range(PREDICTION_YEARS).prefix("year "));
            });
        }
        Answer::Poll(choice) => poll::input(ui, question, choice),
    }
}

fn done(
    ui: &mut egui::Ui,
    survey: &Survey,
    user: &str,
    results: Option<&Results>,
    scores: Option<&[Score]>,
) -> Option<Action> {
    survey_heading(ui, survey);
    ui.label(format!("Thanks {user}, your answers are saved."));
    let action = ui
        .button("Choose another survey")
        .clicked()
        .then_some(Action::ChooseAnotherSurvey);

    if survey.scored {
        ui.separator();
        ui.label("Scores");
        match scores {
            Some(scores) => scoreboard(ui, scores, survey.questions.len()),
            None => {
                ui.spinner();
            }
        }
    }

    ui.separator();
    ui.heading("All answers");
    let Some(results) = results else {
        ui.spinner();
        return action;
    };
    egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
        for question in &survey.questions {
            ui.strong(&question.text);
            let answers = results.get(&question.id).map_or(&[][..], Vec::as_slice);
            if answers.is_empty() {
                ui.label("No answers yet");
            }
            match question.kind {
                AnswerKind::PredictionDate => {
                    let dates =
                        timeline::extract_and_sort_dates(answers.iter().map(|a| (Some(a.user.as_str()), &a.answer)));
                    if !dates.is_empty() {
                        timeline::draw(ui, &dates, Some(user));
                    }
                }
                AnswerKind::Text => {
                    for user_answer in answers {
                        if let Answer::Text(text) = &user_answer.answer {
                            ui.label(format!("{}: {text}", user_answer.user));
                        }
                    }
                }
                AnswerKind::Poll => poll::draw(ui, &poll::rows(question, answers)),
            }
            ui.add_space(8.0);
        }
    });
    action
}

/// The ranking of a scored survey, one line per participant.
fn scoreboard(ui: &mut egui::Ui, scores: &[Score], out_of: usize) {
    for (rank, score) in scores.iter().enumerate() {
        ui.label(score_line(rank + 1, score, out_of));
    }
}

/// `1. Ann: 2 of 3`
fn score_line(rank: usize, score: &Score, out_of: usize) -> String {
    format!("{rank}. {}: {} of {out_of}", score.user, score.points)
}

fn survey_heading(ui: &mut egui::Ui, survey: &Survey) {
    ui.heading(&survey.title);
    ui.label(&survey.description);
    if let Some(link) = &survey.link {
        ui.hyperlink(link);
    }
    ui.add_space(4.0);
}

fn footer(ui: &mut egui::Ui) {
    let small = |text: &str| RichText::new(text).font(FontId::proportional(8.0));
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label(small("Made by Sam Jaques. Powered by "));
        ui.hyperlink_to(small("egui"), "https://github.com/emilk/egui");
        ui.label(small(" and "));
        ui.hyperlink_to(
            small("eframe"),
            "https://github.com/emilk/egui/tree/master/crates/eframe",
        );
        ui.label(small("."));
    });
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use egui::accesskit::Role;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable as _;

    use super::*;

    /// The rectangle the content of a `card` gets in a window of `width`.
    fn content_rect(width: f32) -> egui::Rect {
        let rect = Rc::new(Cell::new(egui::Rect::NOTHING));
        let seen = Rc::clone(&rect);
        let mut harness = Harness::builder()
            .with_size(egui::vec2(width, 400.0))
            .build_ui(move |ui| card(ui, |ui| seen.set(ui.max_rect())));
        harness.run();
        rect.get()
    }

    #[test]
    fn card_is_centered_and_not_wider_than_its_maximum() {
        let window = 1200.0;
        let content = content_rect(window);

        assert!((content.center().x - window / 2.0).abs() < 1.0, "{content:?}");
        assert!(content.width() <= CARD_MAX_WIDTH, "{content:?}");
    }

    #[test]
    fn card_fills_a_narrow_window() {
        let window = CARD_MIN_WIDTH + 40.0;
        let content = content_rect(window);

        assert!(
            content.width() > window - 4.0 * f32::from(CARD_PADDING) - 20.0,
            "{content:?}"
        );
        assert!(content.width() < window, "{content:?}");
    }

    #[test]
    fn card_is_not_squashed_below_its_minimum_width() {
        let narrow = content_rect(CARD_MIN_WIDTH - 150.0);
        let minimum = content_rect(CARD_MIN_WIDTH);

        assert!((narrow.width() - minimum.width()).abs() < 1.0, "{narrow:?} {minimum:?}");
    }

    #[test]
    fn card_scrolls_sideways_when_the_window_is_narrower_than_its_minimum_width() {
        let left = Rc::new(Cell::new(0.0_f32));
        let seen = Rc::clone(&left);
        let mut harness = Harness::builder()
            .with_size(egui::vec2(CARD_MIN_WIDTH - 150.0, 300.0))
            .build_ui(move |ui| card(ui, |ui| seen.set(ui.min_rect().left())));
        harness.run();
        let before = left.get();

        harness.hover_at(egui::pos2(100.0, 150.0));
        harness.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(-100.0, 0.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        });
        harness.run();

        assert!(left.get() < before - 50.0, "before: {before}, after: {}", left.get());
    }

    #[test]
    fn submit_ends_where_the_name_fields_end() {
        let mut state = State::default();
        let mut harness = Harness::builder()
            .with_size(egui::vec2(600.0, 400.0))
            .build_ui(move |ui| {
                card(ui, |ui| start(ui, &mut state, None));
            });
        harness.run();

        let right = |role, label: &str| harness.get_by_role_and_label(role, label).rect().right();
        let submit = right(Role::Button, "Submit");
        assert!((submit - right(Role::TextInput, "First name:")).abs() < 1.0, "{submit}");
        assert!((submit - right(Role::TextInput, "Last name:")).abs() < 1.0, "{submit}");
    }

    #[test]
    fn card_scrolls_when_the_window_is_smaller_than_its_content() {
        let top = Rc::new(Cell::new(0.0_f32));
        let seen = Rc::clone(&top);
        let mut harness = Harness::builder()
            .with_size(egui::vec2(600.0, 300.0))
            .build_ui(move |ui| {
                card(ui, |ui| {
                    ui.allocate_space(egui::vec2(10.0, 1000.0));
                    seen.set(ui.min_rect().top());
                });
            });
        harness.run();
        let before = top.get();

        harness.hover_at(egui::pos2(300.0, 150.0));
        harness.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -200.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        });
        harness.run();

        assert!(top.get() < before - 100.0, "before: {before}, after: {}", top.get());
    }
}
