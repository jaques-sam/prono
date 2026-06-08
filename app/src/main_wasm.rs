#![warn(clippy::all, rust_2018_idioms)]

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use log::error;

static BACKEND_URL_PROD: &str = "https://prono.samagali.myds.me";
static BACKEND_URL_DEV: &str = "http://localhost:8081";

fn backend_url() -> &'static str {
    let hash = web_sys::window()
        .and_then(|w| w.location().hash().ok())
        .unwrap_or_default();
    if hash.contains("dev") {
        BACKEND_URL_DEV
    } else {
        BACKEND_URL_PROD
    }
}

struct ApiThroughRest {
    base_url: String,
    survey: prono_api::Survey,
    device_id: String,
    /// Number of pending write operations (add_user, add_answer).
    /// `all_answers` waits until this reaches 0 before fetching.
    pending_writes: Rc<Cell<u32>>,
    /// Question IDs with an in-flight GET request (prevents duplicate fetches).
    in_flight: Rc<RefCell<HashSet<String>>>,
    /// Cached answers returned from the server.
    cached_all_answers: Rc<RefCell<HashMap<String, Vec<(String, prono_api::Answer)>>>>,
}

impl ApiThroughRest {
    fn new(base_url: String, survey: prono_api::Survey, device_id: String) -> Self {
        Self {
            base_url,
            survey,
            device_id,
            pending_writes: Rc::new(Cell::new(0)),
            in_flight: Rc::new(RefCell::new(HashSet::new())),
            cached_all_answers: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    fn begin_write(&self) {
        self.pending_writes.set(self.pending_writes.get() + 1);
    }

    fn end_write(pending: &Rc<Cell<u32>>) {
        pending.set(pending.get().saturating_sub(1));
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

    fn add_answers(&mut self, user: &str, answers: Vec<(String, prono_api::Answer)>) {
        let url = format!("{}/api/survey/answers", self.base_url);
        let body = serde_json::json!({
            "user": user,
            "answers": answers,
        });
        let body_str = body.to_string();
        let device_id = self.device_id.clone();
        let pending = Rc::clone(&self.pending_writes);
        self.begin_write();

        wasm_bindgen_futures::spawn_local(async move {
            match gloo_net::http::Request::post(&url)
                .header("Content-Type", "application/json")
                .header("Authorization", &format!("Bearer {}", prono_api::API_KEY))
                .header("X-Device-Id", &device_id)
                .body(body_str)
                .expect("Failed to build request body")
                .send()
                .await
            {
                Ok(resp) if !resp.ok() => {
                    error!("add_answers failed: HTTP {} - {}", resp.status(), resp.status_text());
                }
                Err(e) => error!("add_answers network error: {e}"),
                _ => {}
            }
            Self::end_write(&pending);
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

        // Don't fetch while writes (add_user/add_answer) are still in flight
        if self.pending_writes.get() > 0 {
            return Vec::new();
        }

        // Don't spawn duplicate fetches for the same question
        if !self.in_flight.borrow_mut().insert(question_id.clone()) {
            return Vec::new();
        }

        let url = format!("{}/api/survey/answers/{question_id}", self.base_url);
        let cache = Rc::clone(&self.cached_all_answers);
        let in_flight = Rc::clone(&self.in_flight);
        let qid = question_id.clone();

        wasm_bindgen_futures::spawn_local(async move {
            match gloo_net::http::Request::get(&url).send().await {
                Ok(resp) => match resp.json::<Vec<(String, prono_api::Answer)>>().await {
                    Ok(answers) => {
                        if !answers.is_empty() {
                            cache.borrow_mut().insert(qid.clone(), answers);
                        }
                    }
                    Err(e) => error!("Failed to parse all_answers response: {e}"),
                },
                Err(e) => error!("Failed to fetch all_answers: {e}"),
            }
            // Allow retry on next repaint
            in_flight.borrow_mut().remove(&qid);
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
        let base_url = backend_url();
        let survey = match gloo_net::http::Request::get(&format!("{base_url}/api/survey"))
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

        let identity = crate::adapters::identity_wasm::WasmIdentity::load_or_create();
        let device_id = prono_api::Identity::device_id(&identity).to_string();
        let api = ApiThroughRest::new(base_url.to_string(), survey, device_id);

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(move |cc: &eframe::CreationContext<'_>| Ok(Box::new(crate::App::new(cc, api, None)))),
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
