#![warn(clippy::all, rust_2018_idioms)]

/// # Panics
///
/// - if another used library has already initialized a global logger
/// - if the app icon cannot be loaded
pub fn main() {
    struct ApiThroughRest();

    impl prono_api::Surveys for ApiThroughRest {
        fn empty_survey(&self) -> prono_api::Survey {
            todo!()
        }

        fn add_answer(&mut self, _user: &str, _question_id: String, _answer: prono_api::Answer) {
            todo!()
        }

        fn response(&self, _user: &str, _id: u64) -> Option<prono_api::Survey> {
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

        let api = ApiThroughRest();

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(move |cc: &eframe::CreationContext<'_>| Ok(Box::new(crate::App::new(cc, api)))),
            )
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

        // TODO [13] Implement client here
    });
}
