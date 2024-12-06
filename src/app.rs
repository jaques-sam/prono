use egui::TextEdit;
use serde::{Deserialize, Serialize};

use crate::{config, Answer, Survey};

static INIT_ANSWER_HINT: &str = "give your expected date";
static SURVEY_CONFIG: &str = include_str!("./configurations/survey_spacex_starship.json");

#[derive(Deserialize, Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct App {
    user_name: String,
    survey: Survey,
}

impl Default for App {
    fn default() -> Self {
        Self {
            user_name: String::new(),
            survey: config::Survey::create_from_file(SURVEY_CONFIG).into(),
        }
    }
}

impl App {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }

    fn clear(&mut self) {
        self.survey.clear();
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
            ui.heading("Guess SpaceX Starship achievements");

            ui.spacing();

            ui.hyperlink_to("SpaceX Starship", "http://www.spacex.com");

            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.add(TextEdit::singleline(&mut self.user_name).hint_text("Please fill in your name"));
            });

            static QUESTIONS: [&str; 4] = [
                "First cargo only Moon landing",
                "First man on the Moon landing",
                "First cargo only Mars landing",
                "First man on Mars landing",
            ];

            for (i, &question) in QUESTIONS.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(question);
                    match &mut self.survey.questions[i].answer {
                        Answer::Text(answer) => {
                            ui.add(TextEdit::singleline(answer).hint_text(INIT_ANSWER_HINT));
                        }
                        // FIXME add 2 synced combo boxes
                        Answer::PredictionDate { day: _, month, year } => {
                            let month_str = month.to_string();
                            ui.add(TextEdit::singleline(&mut month_str.clone()).hint_text("month"));
                            let year_str = year.to_string();
                            ui.add(TextEdit::singleline(&mut year_str.clone()).hint_text("year"));
                        }
                    }
                });
            }

            ui.horizontal(|ui| {
                if ui.button("Reset").clicked() {
                    self.clear();
                }
                if ui.button("Submit").clicked() {
                    todo!("Submit answers");
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
