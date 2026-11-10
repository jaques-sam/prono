//! Draws the [`State`] and reports what the participant clicked. All rules
//! live in `prono_api` and [`State`]; this module only renders.

use egui::{FontId, RichText, TextEdit};
use prono_api::{Answer, AnswerKind, PREDICTION_YEARS, Results, Survey};

use crate::state::{Action, Phase, State};
use crate::timeline;

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
            let survey = state.survey();
            let phase_action = match (&mut state.phase, survey) {
                (Phase::Answering { answers, .. }, Some(survey)) => answering(ui, survey, answers, busy),
                (Phase::Done { .. }, Some(survey)) => done(ui, survey, &state.user_name, state.results.as_ref()),
                _ => start(ui, &mut state.user_name),
            };
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

fn start(ui: &mut egui::Ui, user_name: &mut String) -> Option<Action> {
    ui.horizontal(|ui| {
        ui.label("Your name:");
        ui.add(TextEdit::singleline(user_name).hint_text("Please fill in your name"));
    });
    ui.add_space(8.0);
    ui.label("Choose a survey:");
    let mut action = None;
    for survey in prono_api::surveys() {
        if ui.button(&survey.description).clicked() {
            action = Some(Action::StartSurvey(survey.id));
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
                ui.label(&question.text);
                answer_input(ui, answer);
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
            if busy {
                ui.spinner();
            }
            action
        })
        .inner;

    ui.separator();
    ui.label("Timeline of your predictions");
    let dates = timeline::extract_and_sort_dates(answers.iter().map(|answer| (None, answer)));
    timeline::draw(ui, &dates);

    action
}

fn answer_input(ui: &mut egui::Ui, answer: &mut Answer) {
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
    }
}

fn done(ui: &mut egui::Ui, survey: &Survey, user_name: &str, results: Option<&Results>) -> Option<Action> {
    survey_heading(ui, survey);
    ui.label(format!("Thanks {user_name}, your answers are saved."));
    let action = ui
        .button("Choose another survey")
        .clicked()
        .then_some(Action::ChooseAnotherSurvey);

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
            }
            ui.add_space(8.0);
        }
    });
    action
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
