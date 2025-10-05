use egui::TextEdit;
use serde::{Deserialize, Serialize};

use crate::{Answer, Question, Survey};

static INIT_ANSWER_HINT: &str = "give your expected date";

fn update_questions(ui: &mut egui::Ui, questions: &mut Vec<Question>) {
    for question in questions {
        ui.horizontal(|ui| {
            ui.label(&question.text);
            match &mut question.answer {
                Answer::Text(answer) => {
                    ui.add(TextEdit::singleline(answer).hint_text(INIT_ANSWER_HINT));
                }
                Answer::PredictionDate { day: _, month, year } => {
                    let mut month_str = month.to_string();
                    ui.add(TextEdit::singleline(&mut month_str).hint_text("month"));
                    *month = month_str.parse().unwrap_or_default();

                    let mut year_str = year.to_string();
                    ui.add(TextEdit::singleline(&mut year_str).hint_text("year"));
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
    survey: Option<Survey>,
    #[serde(skip)]
    prono: Option<Box<dyn prono::Prono>>,
}

impl App {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>, prono: impl prono::Prono + 'static) -> Self {
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

    fn clear(&mut self) {
        if let Some(survey) = &mut self.survey {
            survey.clear();
        }
    }

    fn update_survey(&mut self, ui: &mut egui::Ui) {
        if let Some(survey) = &mut self.survey {
            ui.heading(&survey.description);
            ui.spacing();
            ui.hyperlink_to("SpaceX Starship", "http://www.spacex.com"); // TODO [4]: move to survey
            update_questions(ui, &mut survey.questions);
            // TODO [5]: update survey to api
        } else if ui.button("Start survey").clicked() {
            self.survey = Some(
                self.prono
                    .as_ref()
                    .expect("no prono API adapter set")
                    .empty_survey()
                    .into(),
            );
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

            egui::menu::bar(ui, |ui| {
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

            ui.horizontal(|ui| {
                ui.label("Username:");
                ui.add(TextEdit::singleline(&mut self.user_name).hint_text("Please fill in your name"));
            });

            self.update_survey(ui);

            ui.horizontal(|ui| {
                if ui.button("Reset").clicked() {
                    self.clear();
                }
                if ui.button("Submit").clicked() {
                    let survey = self.survey.take().expect("survey to submit");
                    for question in survey.questions {
                        self.prono.as_mut().expect("no prono API adapter set").add_answer(
                            &self.user_name,
                            question.id,
                            question.answer.into(),
                        );
                    }
                }
            });

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
