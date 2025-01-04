use async_trait::async_trait;
use log::info;
use mysql_async::Opts;
use prono::api::PronoApi;
use tokio::runtime;

use crate::{Config, DbAnswer, DB};

pub struct MysqlDb {
    rt: runtime::Runtime,
}

impl MysqlDb {
    pub fn new(secure_config: Config) -> Self {
        let db = Self {
            rt: runtime::Builder::new_current_thread().enable_all().build().unwrap(),
        };
        db.initialize(secure_config);
        db
    }
}

impl PronoApi for MysqlDb {
    fn answer(&self, user: &str, question_id: u64) -> prono::api::Answer {
        log::info!("DB: user {user} asks for answer for question id={question_id})");
        todo!()
    }

    fn response(&self, user: &str, survey_id: u64) -> Option<prono::api::Survey> {
        log::info!("DB: user {user} asks for response for survey id={survey_id})");
        todo!()
    }
}

#[async_trait]
impl DB for MysqlDb {
    fn initialize(&self, secure_config: Config) {
        info!("Initializing Mysql db {}...", secure_config.db_name);

        self.rt.spawn(async move {
            let database_url = secure_config.construct_url();
            let database_url = database_url.unsecure();
            let pool = mysql_async::Pool::new(Opts::from_url(database_url).expect("catch this error"));

            let mut _conn = pool.get_conn().await.expect("catch this error");
        });
    }

    async fn add_answer(&mut self, user: u64, question_id: u16, _answer: DbAnswer) {
        log::info!("DB: user {user} adds answer for question id={question_id})");
    }
}
