//! Draws the [`State`] and reports what the participant clicked. All rules
//! live in `prono_api` and [`State`]; this module only renders.

use egui::{FontId, RichText, TextEdit};
use prono_api::{Access, Answer, AnswerKind, PREDICTION_YEARS, Question, Results, Score, Survey};

use crate::state::{Action, Phase, State, SurveyList};
use crate::{poll, timeline};

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
        .show(ui, |ui| {
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

/// A single-line text field with a `label` the screen readers know it by.
fn labelled_text_edit(ui: &mut egui::Ui, label: &str, hint: &str, text: &mut String, password: bool) {
    ui.horizontal(|ui| {
        let label = ui.label(label);
        ui.add(TextEdit::singleline(text).hint_text(hint).password(password))
            .labelled_by(label.id);
    });
}

fn start(ui: &mut egui::Ui, state: &mut State, warning: Option<&str>) -> Option<Action> {
    labelled_text_edit(ui, "First name:", "Your first name", &mut state.first_name, false);
    labelled_text_edit(ui, "Last name:", "Your last name", &mut state.last_name, false);
    if let Some(warning) = warning {
        ui.colored_label(ui.visuals().warn_fg_color, warning);
    }
    ui.add_space(8.0);
    // Taken out of the state, which the survey list borrows meanwhile.
    let mut pass_phrase = std::mem::take(&mut state.pass_phrase);
    let action = match state.survey_list() {
        SurveyList::NeedName => {
            ui.label("Fill in your first and last name to see your surveys.");
            None
        }
        SurveyList::Waiting => {
            ui.spinner();
            None
        }
        SurveyList::Failed => {
            ui.label("Your surveys could not be loaded. Change your name to try again.");
            None
        }
        SurveyList::Ready([]) => {
            ui.label("No surveys are available for you.");
            None
        }
        SurveyList::Ready(surveys) => {
            ui.label("Choose a survey:");
            let mut action = None;
            for survey in surveys {
                ui.horizontal(|ui| {
                    if ui.button(&survey.description).clicked() {
                        action = Some(Action::StartSurvey(survey.id));
                    }
                    // Asked here, with the choice, so it is known before the
                    // questions open.
                    if survey.access == Access::PassPhrase {
                        labelled_text_edit(
                            ui,
                            "Pass phrase:",
                            "The pass phrase of this survey",
                            &mut pass_phrase,
                            true,
                        );
                    }
                });
            }
            action
        }
    };
    state.pass_phrase = pass_phrase;
    action
}

fn answering(ui: &mut egui::Ui, survey: &Survey, answers: &mut [Answer], busy: bool) -> Option<Action> {
    survey_heading(ui, survey);

    egui::Grid::new("questions")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            for (question, answer) in survey.questions.iter().zip(answers.iter_mut()) {
                ui.label(&question.text);
                answer_input(ui, question, answer);
                ui.end_row();
            }
        });

    let action = ui
        .horizontal(|ui| {
            let mut action = None;
            if ui.add_enabled(!busy, egui::Button::new("Reset")).clicked() {
                action = Some(Action::ResetAnswers);
            }
            if ui.add_enabled(!busy, egui::Button::new("Submit")).clicked() {
                action = Some(Action::Submit);
            }
            if ui.add_enabled(!busy, egui::Button::new("Change name")).clicked() {
                action = Some(Action::ChangeName);
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
        ui.label("Timeline of your predictions");
        let dates = timeline::extract_and_sort_dates(answers.iter().map(|answer| (None, answer)));
        timeline::draw(ui, &dates);
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
    ui.label("All answers");
    let Some(results) = results else {
        ui.spinner();
        return action;
    };
    egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
        for question in &survey.questions {
            ui.heading(&question.text);
            let answers = results.get(&question.id).map_or(&[][..], Vec::as_slice);
            if answers.is_empty() {
                ui.label("No answers yet");
            }
            match question.kind {
                AnswerKind::PredictionDate => {
                    let dates =
                        timeline::extract_and_sort_dates(answers.iter().map(|a| (Some(a.user.as_str()), &a.answer)));
                    if !dates.is_empty() {
                        timeline::draw(ui, &dates);
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
    ui.heading(&survey.description);
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
