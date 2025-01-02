#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

// CLEAN ARCHITECTURE
mod adapters;
mod entities;
pub(crate) use adapters::*;
pub(crate) use entities::*;

use eframe::AppCreator;
use prono::{api, ReadConfig};
use std::path::Path;

static CONFIG_FILENAME: &str = "secure_config.toml";

#[cfg(not(target_arch = "wasm32"))]
static DB_NAME: &str = "db_prono";

fn build_app<'a>(prono: impl prono::Prono + 'static) -> AppCreator<'a> {
    Box::new(|cc| Ok(Box::new(crate::App::new(cc, prono))))
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    let db_config: prono_db::Config = ConfigRead {}.read(Path::new(CONFIG_FILENAME)).db.into();
    let db: Box<dyn api::PronoApi> = Box::new(prono_db::MysqlDb::new(db_config));

    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0])
            .with_icon(
                // NOTE: Adding an icon is optional
                eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon-256.png")[..])
                    .expect("Failed to load icon"),
            ),
        ..Default::default()
    };
    let prono_lib = prono::PronoLib::new(Some(db));
    eframe::run_native("eframe template", native_options, build_app(prono_lib))
}

#[cfg(target_arch = "wasm32")]
fn main() {
    struct ApiThroughRest();
    let _config = ConfigRead {}.read(Path::new(CONFIG_FILENAME));

    impl api::PronoApi for ApiThroughRest {
        fn answer(&self, _user: u64, _id: u16) -> api::Answer {
            todo!()
        }
    }

    impl prono::Prono for ApiThroughRest {
        fn survey(&self) -> api::Survey {
            todo!()
        }
    }

    use eframe::wasm_bindgen::JsCast as _;

    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window().expect("No window").document().expect("No document");

        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Failed to find the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not a HtmlCanvasElement");

        let start_result = eframe::WebRunner::new()
            .start(canvas, web_options, build_app(ApiThroughRest()))
            .await;

        // Remove the loading text and spinner:
        if let Some(loading_text) = document.get_element_by_id("loading_text") {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                }
                Err(e) => {
                    loading_text.set_inner_html("<p> The app has crashed. See the developer console for details. </p>");
                    panic!("Failed to start eframe: {e:?}");
                }
            }
        }
    });
}
