use egui::TextEdit;
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

    fn draw_timeline_from_answers(&self, ui: &mut egui::Ui) {
        ui.spacing();
        ui.separator();

        match &self.survey_state {
            SurveyState::InProgress(survey) => {
                ui.label("Timeline of your predictions");
                let answers: Vec<&Answer> = survey.questions.iter().map(|q| &q.answer).collect();
                let timeline_dates = timeline::extract_dates(vec![answers]);
                timeline::draw(ui, &timeline_dates);
            }
            SurveyState::Completed => {
                ui.label("Timeline of all predictions");
                // let answers: Vec<&Answer> = self
                //     .prono
                //     .as_ref()
                //     .expect("no prono API adapter set")
                //     .get_answers() // TODO [13]: Show timeline of all answers
                //     .into_iter()
                //     .map(|a| a.answer)
                //     .collect();
                let timeline_dates = Vec::new(); // timeline::extract_dates(vec![answers]);
                timeline::draw(ui, &timeline_dates);
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
            SurveyState::Completed => {
                ui.label("Survey completed.");
                // TODO [13]: Show timeline of all answers
            }
            SurveyState::NotStarted => {
                if ui.button("Start survey").clicked() {
                    self.survey_state = SurveyState::InProgress(
                        self.prono
                            .as_ref()
                            .expect("no prono API adapter set")
                            .empty_survey()
                            .into(),
                    );
                }
            }
        }
        self.draw_timeline_from_answers(ui);
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
                SurveyState::InProgress(results) => {
                    if ui.button("Reset").clicked() {
                        results.empty();
                    } else if ui.button("Submit").clicked() {
                        for question in results.questions.drain(..) {
                            self.prono.as_mut().expect("no prono API adapter set").add_answer(
                                &self.user_name,
                                question.id,
                                question.answer.into(),
                            );
                        }
                        self.survey_state = SurveyState::Completed;
                    }
                }
                SurveyState::Completed => {
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
