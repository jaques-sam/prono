use log::info;
use mysql_async::{prelude::*, Opts};
use std::sync::Arc;

use prono::api;
use tokio::{runtime, sync::Mutex};

use crate::Config;

struct State {
    connection: mysql_async::Conn,
}

pub struct MysqlDb {
    rt: runtime::Runtime,
    state: Arc<Mutex<Option<State>>>,
}

impl MysqlDb {
    pub fn new(secure_config: Config) -> Self {
        let database_url = secure_config.construct_url();
        let database_url = database_url.unsecure();
        let pool = mysql_async::Pool::new(Opts::from_url(database_url).expect("catch this error"));

        let rt = runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let state: Arc<Mutex<Option<State>>> = Arc::new(Mutex::new(None));
        let state_clone = state.clone();

        rt.block_on(async move {
            let connection = pool.get_conn().await.expect("catch this error");
            *state_clone.lock().await = Some(State { connection });
        });

        Self { rt, state }
    }
}

impl api::PronoApi for MysqlDb {
    fn answer(&self, user: &str, question_id: u64) -> api::Answer {
        info!("DB: user {user} asks for answer for question id={question_id})");
        todo!()
    }

    fn response(&self, user: &str, survey_id: u64) -> Option<api::Survey> {
        info!("DB: user {user} asks for response for survey id={survey_id})");
        None
    }

    fn add_answer(&mut self, user: &str, question_id: String, answer: api::Answer) {
        info!("DB: user {user} adds answer for question id={question_id})");
        let state = self.state.clone();
        let user = user.to_string();

        self.rt.block_on(async move {
            let mut lock = state.lock().await;
            let state = lock.as_mut().expect("catch this error");
            let answer = answer.to_string();

            r"INSERT INTO AnswerResponse (user, question_id, answer)
              VALUES (:user, :question_id, :answer )"
                .with(params! { "user" => user, "question_id" => question_id, "answer" => answer })
                .ignore(&mut state.connection)
                .await
                .expect("catch this error");
        });
    }
}
