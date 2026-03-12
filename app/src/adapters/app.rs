use egui::TextEdit;
use log::debug;
use serde::{Deserialize, Serialize};

use super::timeline;
use crate::{Answer, Question, SurveyState};

static INIT_ANSWER_HINT: &str = "your answer here";

fn update_questions(ui: &mut egui::Ui, questions: &mut Vec<Question>) {
    for question in questions {
        ui.horizontal(|ui| {
            ui.label(&question.text);
            match &mut question.answer {
                Answer::Text(answer) => {
                    ui.add(TextEdit::singleline(answer)).on_hover_text(INIT_ANSWER_HINT);
                }
                Answer::PredictionDate { day: _, month, year } => {
                    let mut month_str = month.to_string();
                    ui.add(TextEdit::singleline(&mut month_str).char_limit(2).desired_width(15.0))
                        .on_hover_text("month");
                    *month = month_str.parse().unwrap_or_default();

                    let mut year_str = year.to_string();
                    ui.add(TextEdit::singleline(&mut year_str).char_limit(4).desired_width(35.0))
                        .on_hover_text("year");
                    *year = year_str.parse().unwrap_or_default();
                }
            }
        });
    }
}

#[derive(Default, Deserialize, Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct App {
    user_name: String,
    survey_state: SurveyState,
    #[serde(skip)]
    prono: Option<Box<dyn prono_api::Surveys>>,
    #[serde(skip)]
    error_message: Option<String>,
}

impl App {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>, prono: impl prono_api::Surveys + 'static) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        let mut app = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or(Self::default())
        } else {
            Self::default()
        };
        app.prono = Some(Box::new(prono));
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
        self.survey_state = SurveyState::Completed(survey);
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
                        let Some(prono) = self.prono.as_ref() else {
                            return;
                        };
                        let all_answers = prono.all_answers(question.id.clone());
                        let all_answers: Vec<(Option<&String>, Answer)> = all_answers
                            .iter()
                            .map(|(user, answer)| (Some(user), answer.clone().into()))
                            .collect();
                        if all_answers.is_empty() {
                            ui.label("No predictions for this question");
                        } else {
                            debug!("Number of answers for Q:{}: {}", question.id, all_answers.len());
                            let timeline_dates = timeline::extract_and_sort_dates(all_answers);
                            timeline::draw(ui, &timeline_dates);
                        }
                        ui.spacing();
                    }
                });
            }
            SurveyState::NotStarted => {}
        }
    }

    fn update_survey(&mut self, ui: &mut egui::Ui) {
        match &mut self.survey_state {
            SurveyState::InProgress(survey) => {
                ui.heading(&survey.description);
                ui.spacing();
                ui.hyperlink_to("SpaceX Starship", "http://www.spacex.com"); // TODO [4]: move to survey

                update_questions(ui, &mut survey.questions);

                ui.spacing();
            }
            SurveyState::Completed(_survey) => {
                ui.label("Survey completed.");
            }
            SurveyState::NotStarted => {
                if ui.button("Start survey").clicked() {
                    if let Some(prono) = self.prono.as_ref() {
                        self.survey_state = SurveyState::InProgress(prono.empty_survey().into());
                    } else {
                        self.error_message = Some("No backend connection available".to_string());
                    }
                }
            }
        }
        self.draw_timeline_from_answers(ui);
    }

    fn show_error_overlay(&mut self, ctx: &egui::Context) {
        if let Some(ref msg) = self.error_message {
            let screen = ctx.content_rect();
            let msg = msg.clone();

            // Semi-transparent background covering the entire screen
            egui::Area::new(egui::Id::new("error_overlay_bg"))
                .fixed_pos(screen.min)
                .show(ctx, |ui| {
                    let (rect, response) = ui.allocate_exact_size(screen.size(), egui::Sense::click());
                    ui.painter()
                        .rect_filled(rect, 0.0, egui::Color32::from_black_alpha(160));
                    if response.clicked() {
                        self.error_message = None;
                    }
                });

            // Centered error message on top
            egui::Area::new(egui::Id::new("error_overlay_msg"))
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.heading("Error");
                            ui.add_space(8.0);
                            ui.label(&msg);
                            ui.add_space(8.0);
                            if ui.button("OK").clicked() {
                                self.error_message = None;
                            }
                        });
                    });
                });
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
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

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
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading("Prono");

            ui.horizontal(|ui| match &mut self.survey_state {
                SurveyState::NotStarted => {
                    ui.label("Username:");
                    ui.add(TextEdit::singleline(&mut self.user_name).hint_text("Please fill in your name"));
                }
                SurveyState::InProgress(_) => {
                    if ui.button("Reset").clicked() {
                        if let SurveyState::InProgress(survey) = &mut self.survey_state {
                            survey.empty();
                        }
                    } else if ui.button("Submit").clicked() {
                        self.submit();
                    }
                }
                SurveyState::Completed(_) => {
                    if cfg!(debug_assertions) && ui.button("Survey again").clicked() {
                        self.user_name.clear();
                        self.survey_state = SurveyState::NotStarted;
                    }
                }
            });

            self.update_survey(ui);

            ui.separator();

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                powered_by_egui_and_eframe(ui);
                egui::warn_if_debug_build(ui);
            });
        });

        self.show_error_overlay(ctx);
    }
}

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("Made by Sam Jaques. ");
        ui.label("Powered by ");
        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
        ui.label(" and ");
        ui.hyperlink_to("eframe", "https://github.com/emilk/egui/tree/master/crates/eframe");
        ui.label(".");
    });
}

#[cfg(test)]
mod tests {
    use prono_api::MockSurveys;

    use crate::Survey;

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
