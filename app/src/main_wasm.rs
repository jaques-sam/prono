#![warn(clippy::all, rust_2018_idioms)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use log::error;

// TODO: make this configurable
static BACKEND_URL: &str = "http://127.0.0.1:8081";

struct ApiThroughRest {
    base_url: String,
    survey: prono_api::Survey,
    cached_all_answers: Rc<RefCell<HashMap<String, Vec<(String, prono_api::Answer)>>>>,
}

impl ApiThroughRest {
    fn new(base_url: String, survey: prono_api::Survey) -> Self {
        Self {
            base_url,
            survey,
            cached_all_answers: Rc::new(RefCell::new(HashMap::new())),
        }
    }
}

impl prono_api::Surveys for ApiThroughRest {
    fn empty_survey(&self) -> prono_api::Survey {
        prono_api::Survey {
            id: self.survey.id,
            description: self.survey.description.clone(),
            questions: self
                .survey
                .questions
                .iter()
                .map(|q| prono_api::Question {
                    id: q.id.clone(),
                    answer: q.answer.clone(),
                    text: q.text.clone(),
                })
                .collect(),
        }
    }

    fn add_answer(&mut self, user: &str, question_id: String, answer: prono_api::Answer) {
        let url = format!("{}/api/survey/answer", self.base_url);
        let body = serde_json::json!({
            "user": user,
            "question_id": question_id,
            "answer": answer,
        });
        let body_str = body.to_string();

        wasm_bindgen_futures::spawn_local(async move {
            let result = gloo_net::http::Request::post(&url)
                .header("Content-Type", "application/json")
                .body(body_str)
                .expect("Failed to build request body")
                .send()
                .await;
            if let Err(e) = result {
                error!("Failed to submit answer: {e}");
            }
        });
    }

    fn response(&self, _user: &str, _id: u64) -> Option<prono_api::Survey> {
        None
    }

    fn all_answers(&self, question_id: String) -> Vec<(String, prono_api::Answer)> {
        // Return cached results if available
        if let Some(cached) = self.cached_all_answers.borrow().get(&question_id) {
            return cached.clone();
        }

        // Spawn async fetch and cache the result
        let url = format!("{}/api/survey/answers/{question_id}", self.base_url);
        let cache = Rc::clone(&self.cached_all_answers);
        let qid = question_id.clone();

        wasm_bindgen_futures::spawn_local(async move {
            match gloo_net::http::Request::get(&url).send().await {
                Ok(resp) => match resp.json::<Vec<(String, prono_api::Answer)>>().await {
                    Ok(answers) => {
                        cache.borrow_mut().insert(qid, answers);
                    }
                    Err(e) => error!("Failed to parse all_answers response: {e}"),
                },
                Err(e) => error!("Failed to fetch all_answers: {e}"),
            }
        });

        Vec::new()
    }
}

/// # Panics
///
/// - if another used library has already initialized a global logger
/// - if the app icon cannot be loaded
pub fn main() {
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

        // Pre-fetch the survey from the backend before starting the app
        let survey = match gloo_net::http::Request::get(&format!("{BACKEND_URL}/api/survey"))
            .send()
            .await
        {
            Ok(resp) => match resp.json::<prono_api::Survey>().await {
                Ok(survey) => survey,
                Err(e) => {
                    error!("Failed to parse survey: {e}");
                    return;
                }
            },
            Err(e) => {
                error!("Failed to fetch survey from backend: {e}");
                return;
            }
        };

        let api = ApiThroughRest::new(BACKEND_URL.to_string(), survey);

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
                Ok(()) => {
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
