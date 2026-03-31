use log::debug;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{error_overlay, footer, survey_ui, timeline};
use crate::{Answer, SurveyState};

#[derive(Default, Deserialize, Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct App {
    user_name: String,
    survey_state: SurveyState,
    #[serde(skip)]
    prono: Option<Box<dyn prono_api::Surveys>>,
    #[serde(skip)]
    error_message: Option<String>,
    /// Cached answers fetched once when survey is completed.
    #[serde(skip)]
    cached_answers: HashMap<String /*question_id*/, Vec<(String, Answer)>>,
}

impl App {
    /// Called once before the first frame.
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        prono: impl prono_api::Surveys + 'static,
        initial_error: Option<String>,
    ) -> Self {
        // Load previous app state (if any).
        let mut app = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or(Self::default())
        } else {
            Self::default()
        };
        app.prono = Some(Box::new(prono));
        app.error_message = initial_error;
        app
    }

    fn submit(&mut self) {
        let survey = match std::mem::replace(&mut self.survey_state, SurveyState::NotStarted) {
            SurveyState::InProgress(s) => s,
            other => {
                self.survey_state = other;
                return;
            }
        };

        let Some(prono) = self.prono.as_mut() else {
            self.error_message = Some("No backend connection available".to_string());
            self.survey_state = SurveyState::InProgress(survey);
            return;
        };

        for question in &survey.questions {
            prono.add_answer(&self.user_name, question.id.clone(), question.answer.clone().into());
        }

        self.cached_answers.clear();
        for question in &survey.questions {
            let all_answers = prono.all_answers(question.id.clone());
            let converted: Vec<(String, Answer)> = all_answers
                .into_iter()
                .map(|(user, answer)| (user, answer.into()))
                .collect();
            self.cached_answers.insert(question.id.clone(), converted);
        }

        self.survey_state = SurveyState::Completed(survey);
    }

    fn reset_survey(&mut self) {
        match &mut self.survey_state {
            SurveyState::InProgress(survey) => {
                survey.empty();
            }
            SurveyState::Completed(_) => {
                self.user_name.clear();
                self.survey_state = SurveyState::NotStarted;
                self.cached_answers.clear();
            }
            SurveyState::NotStarted => {}
        }
    }

    fn draw_timeline_from_answers(&self, ui: &mut egui::Ui) {
        ui.spacing();
        ui.separator();

        match &self.survey_state {
            SurveyState::InProgress(survey) => {
                ui.label("Timeline of your predictions");
                let answers = survey.questions.iter().map(|q| (None, q.answer.clone())).collect();
                let timeline_dates = timeline::extract_and_sort_dates(answers);
                timeline::draw(ui, &timeline_dates);
            }
            SurveyState::Completed(survey) => {
                ui.label("All answers");
                egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
                    for question in &survey.questions {
                        ui.heading(&question.text);

                        // Use cached answers instead of querying database every frame
                        if let Some(cached) = self.cached_answers.get(&question.id) {
                            if cached.is_empty() {
                                ui.label("No predictions for this question");
                            } else {
                                debug!("Number of answers for Q:{}: {}", question.id, cached.len());
                                let all_answers: Vec<(Option<&String>, Answer)> = cached
                                    .iter()
                                    .map(|(user, answer)| (Some(user), answer.clone()))
                                    .collect();
                                let timeline_dates = timeline::extract_and_sort_dates(all_answers);
                                timeline::draw(ui, &timeline_dates);
                            }
                        } else {
                            ui.label("No predictions for this question");
                        }
                        ui.spacing();
                    }
                });
            }
            SurveyState::NotStarted => {}
        }
    }
}

impl eframe::App for App {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(16.0);
                }

                egui::widgets::global_theme_preference_buttons(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Prono");

            let action = ui
                .horizontal(|ui| match &self.survey_state {
                    SurveyState::NotStarted => {
                        survey_ui::render_username_input(ui, &mut self.user_name);
                        survey_ui::SurveyAction::None
                    }
                    SurveyState::InProgress(_) | SurveyState::Completed(_) => {
                        survey_ui::render_survey_controls(ui, &self.survey_state)
                    }
                })
                .inner;

            match action {
                survey_ui::SurveyAction::Reset => self.reset_survey(),
                survey_ui::SurveyAction::Submit => self.submit(),
                survey_ui::SurveyAction::None => {}
            }

            survey_ui::render_survey_content(
                ui,
                &mut self.survey_state,
                self.prono.as_deref(),
                &mut self.error_message,
            );

            self.draw_timeline_from_answers(ui);

            ui.separator();

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                footer::render_footer(ui);
                egui::warn_if_debug_build(ui);
            });
        });

        error_overlay::show_error_overlay(ctx, &mut self.error_message);
    }
}

#[cfg(test)]
mod tests {
    use prono_api::MockSurveys;

    use crate::{Question, Survey};

    use super::*;

    fn make_app(prono: impl prono_api::Surveys + 'static) -> App {
        App {
            prono: Some(Box::new(prono)),
            ..App::default()
        }
    }

    #[test]
    fn submit_transitions_state_to_completed() {
        let mut mock_surveys = MockSurveys::new();
        mock_surveys
            .expect_add_answer()
            .withf(|_user, question_id, answer| {
                question_id == "q1" && answer == &prono_api::Answer::Text("sometime in 2025".to_owned())
            })
            .return_const(());

        mock_surveys.expect_all_answers().returning(|_| {
            vec![(
                "user1".to_string(),
                prono_api::Answer::Text("sometime in 2025".to_owned()),
            )]
        });

        let mut app = make_app(mock_surveys);
        app.survey_state = SurveyState::InProgress(Survey {
            id: 1,
            description: "Test survey".to_string(),
            questions: vec![Question {
                id: "q1".to_string(),
                text: "When will the next launch be?".to_string(),
                answer: Answer::Text("sometime in 2025".to_string()),
            }],
        });

        app.submit();
        assert!(matches!(app.survey_state, SurveyState::Completed(_)));
        assert!(app.cached_answers.contains_key("q1"));
    }

    #[test]
    fn submit_without_adapter_sets_error_message() {
        let mut app = App {
            survey_state: SurveyState::InProgress(Survey {
                id: 1,
                description: "Test".to_string(),
                questions: vec![Question {
                    id: "q1".to_string(),
                    text: "Q?".to_string(),
                    answer: Answer::Text("a".to_string()),
                }],
            }),
            ..App::default()
        };

        app.submit();
        assert!(app.error_message.is_some());
        assert!(matches!(app.survey_state, SurveyState::InProgress(_)));
    }
}
