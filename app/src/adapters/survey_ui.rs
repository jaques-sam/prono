use egui::TextEdit;

use crate::{Answer, Question, SurveyState};

static INIT_ANSWER_HINT: &str = "your answer here";

pub fn render_questions(ui: &mut egui::Ui, questions: &mut [Question]) {
    for question in questions {
        ui.horizontal(|ui| {
            ui.label(&question.text);
            match &mut question.answer {
                Answer::Text(answer) => {
                    ui.add(TextEdit::singleline(answer)).on_hover_text(INIT_ANSWER_HINT);
                }
                Answer::PredictionDate { day: _, month, year } => {
                    ui.add(egui::DragValue::new(month).range(1..=12).prefix("month "))
                        .on_hover_text("1-12");

                    ui.add(egui::DragValue::new(year).range(2024..=2100).prefix("year "))
                        .on_hover_text("2026-2100");
                }
            }
        });
    }
}

pub fn render_survey_content(
    ui: &mut egui::Ui,
    survey_state: &mut SurveyState,
    prono: Option<&dyn prono_api::Surveys>,
    error_message: &mut Option<String>,
) {
    match survey_state {
        SurveyState::NotStarted => {
            if ui.button("Start survey").clicked() {
                if let Some(prono) = prono.as_ref() {
                    *survey_state = SurveyState::InProgress(prono.empty_survey().into());
                } else {
                    *error_message = Some("No backend connection available".to_string());
                }
            }
        }
        SurveyState::InProgress(survey) => {
            ui.heading(&survey.description);
            ui.spacing();
            ui.hyperlink_to("SpaceX Starship", "http://www.spacex.com"); // TODO [4]: move to survey

            render_questions(ui, &mut survey.questions);

            ui.spacing();
        }
        SurveyState::Completed(_survey) => {
            ui.label("Survey completed.");
        }
    }
}

pub enum SurveyAction {
    None,
    Reset,
    Submit,
}

pub fn render_survey_controls(ui: &mut egui::Ui, survey_state: &SurveyState) -> SurveyAction {
    match survey_state {
        SurveyState::InProgress(_) => {
            if ui.button("Reset").clicked() {
                SurveyAction::Reset
            } else if ui.button("Submit").clicked() {
                SurveyAction::Submit
            } else {
                SurveyAction::None
            }
        }
        SurveyState::Completed(_) => {
            #[cfg(debug_assertions)]
            if ui.button("Survey again").clicked() {
                SurveyAction::Reset
            } else {
                SurveyAction::None
            }
            #[cfg(not(debug_assertions))]
            SurveyAction::None
        }
        SurveyState::NotStarted => SurveyAction::None,
    }
}

pub fn render_username_input(ui: &mut egui::Ui, user_name: &mut String) {
    ui.label("Username:");
    ui.add(TextEdit::singleline(user_name).hint_text("Please fill in your name"));
}
