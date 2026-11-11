use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use prono_api::{API_KEY, DEVICE_ID_HEADER, Results, Score, Submission, UserStatus};

pub const PROD_URL: &str = "https://prono.samagali.myds.me";
#[cfg(target_arch = "wasm32")]
pub const DEV_URL: &str = "http://localhost:8081";

/// What the app needs from the Prono backend. Requests never block: they
/// return a [`Pending`] result to poll.
pub trait Backend {
    fn submit(&self, survey_id: u64, submission: &Submission) -> Pending<()>;
    fn results(&self, survey_id: u64) -> Pending<Results>;
    /// The scores of a scored survey, best first.
    fn scores(&self, survey_id: u64) -> Pending<Vec<Score>>;
    /// Whether `user` (the full name) already filled in a survey, and the
    /// surveys they may answer.
    fn user_status(&self, user: &str) -> Pending<UserStatus>;
}

/// The result of a request that may still be in flight. The error is a
/// message for the participant.
pub struct Pending<T>(Arc<Mutex<Option<Result<T, String>>>>);

impl<T> Pending<T> {
    /// A request that already finished with `result`, for fakes.
    pub fn ready(result: Result<T, String>) -> Self {
        Self(Arc::new(Mutex::new(Some(result))))
    }

    /// A request that never finishes, for fakes.
    pub fn never() -> Self {
        Self(Arc::default())
    }

    /// The result, once the request finished.
    pub fn take(&self) -> Option<Result<T, String>> {
        lock(&self.0).take()
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The backend's REST API, the same on desktop and on the web.
pub struct HttpBackend {
    base_url: String,
    device_id: String,
    /// Repainted when a response arrives, so the UI picks it up.
    ctx: egui::Context,
}

impl HttpBackend {
    pub fn new(base_url: String, device_id: String, ctx: egui::Context) -> Self {
        Self {
            base_url,
            device_id,
            ctx,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }

    /// Adds the API key and the device id that identify this app.
    fn authorized(&self, mut request: ehttp::Request) -> ehttp::Request {
        request.headers.insert("Authorization", format!("Bearer {API_KEY}"));
        request.headers.insert(DEVICE_ID_HEADER, &self.device_id);
        request
    }

    fn send<T: Send + 'static>(
        &self,
        request: ehttp::Request,
        parse: fn(&ehttp::Response) -> Result<T, String>,
    ) -> Pending<T> {
        let slot = Arc::default();
        let pending = Pending(Arc::clone(&slot));
        let ctx = self.ctx.clone();
        ehttp::fetch(request, move |response| {
            let result = match response {
                Ok(response) if response.ok => parse(&response),
                Ok(response) => Err(error_message(&response)),
                Err(err) => Err(format!("Cannot reach the server ({err})")),
            };
            *lock(&slot) = Some(result);
            ctx.request_repaint();
        });
        pending
    }
}

impl Backend for HttpBackend {
    fn submit(&self, survey_id: u64, submission: &Submission) -> Pending<()> {
        match ehttp::Request::post_json(self.url(&prono_api::answers_path(survey_id)), submission) {
            Ok(request) => self.send(self.authorized(request), |_| Ok(())),
            Err(err) => Pending(Arc::new(Mutex::new(Some(Err(err.to_string()))))),
        }
    }

    fn results(&self, survey_id: u64) -> Pending<Results> {
        self.send(
            ehttp::Request::get(self.url(&prono_api::answers_path(survey_id))),
            parse_json,
        )
    }

    fn scores(&self, survey_id: u64) -> Pending<Vec<Score>> {
        self.send(
            ehttp::Request::get(self.url(&prono_api::scores_path(survey_id))),
            parse_json,
        )
    }

    fn user_status(&self, user: &str) -> Pending<UserStatus> {
        let request = ehttp::Request::get(self.url(&prono_api::user_path(user)));
        self.send(self.authorized(request), parse_json)
    }
}

fn parse_json<T: serde::de::DeserializeOwned>(response: &ehttp::Response) -> Result<T, String> {
    response
        .json()
        .map_err(|err| format!("Unexpected answer from the server ({err})"))
}

/// The backend sends its error message as a JSON string.
fn error_message(response: &ehttp::Response) -> String {
    response
        .json::<String>()
        .unwrap_or_else(|_| format!("Server error: {} {}", response.status, response.status_text))
}
