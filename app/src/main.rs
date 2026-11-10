#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod device;

use prono_app::client::{self, HttpBackend};
use prono_app::state::State;

struct App {
    state: State,
    backend: HttpBackend,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>, backend_url: String) -> Self {
        let state = cc
            .storage
            .and_then(|storage| eframe::get_value::<State>(storage, eframe::APP_KEY))
            .unwrap_or_default()
            .restored();
        let backend = HttpBackend::new(backend_url, device::device_id(), cc.egui_ctx.clone());
        Self { state, backend }
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.state);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        prono_app::frame(ui, &mut self.state, &self.backend);
    }
}

/// # Errors
///
/// Fails if the graphics context cannot be created.
///
/// # Panics
///
/// If the embedded app icon is invalid.
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with e.g. `RUST_LOG=debug`).

    let backend_url = std::env::var("PRONO_BACKEND_URL").unwrap_or_else(|_| client::PROD_URL.to_string());
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([500.0, 500.0])
            .with_min_inner_size([300.0, 300.0])
            .with_icon(
                eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon-256.png")[..])
                    .expect("Failed to load icon"),
            ),
        ..Default::default()
    };
    eframe::run_native("Prono", options, Box::new(|cc| Ok(Box::new(App::new(cc, backend_url)))))
}

/// # Panics
///
/// If the page has no `the_canvas_id` canvas.
#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let window = web_sys::window().expect("No window");
    // `#dev` in the URL talks to a local backend.
    let dev = window.location().hash().is_ok_and(|hash| hash.contains("dev"));
    let backend_url = if dev { client::DEV_URL } else { client::PROD_URL }.to_string();

    wasm_bindgen_futures::spawn_local(async move {
        let document = window.document().expect("No document");
        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Failed to find the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not a HtmlCanvasElement");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(|cc| Ok(Box::new(App::new(cc, backend_url)))),
            )
            .await;

        // Remove the loading text and spinner:
        if let Some(loading_text) = document.get_element_by_id("loading_text") {
            match start_result {
                Ok(()) => loading_text.remove(),
                Err(e) => {
                    loading_text.set_inner_html("<p> The app has crashed. See the developer console for details. </p>");
                    panic!("Failed to start eframe: {e:?}");
                }
            }
        }
    });
}
