use async_trait::async_trait;
use log::info;
use mysql_async::Opts;
use tokio::runtime;

use crate::prono_db::{self, DbAnswer, DB};

static DB_NAME: &str = "db_prono";

pub struct MysqlDb {
    rt: runtime::Runtime,
}

impl MysqlDb {
    pub fn new(secure_config: prono_db::Config) -> Self {
        let db = Self {
            rt: runtime::Builder::new_multi_thread().enable_all().build().unwrap(),
        };
        db.initialize(secure_config);
        db
    }
}

#[async_trait]
impl DB for MysqlDb {
    fn initialize(&self, secure_config: prono_db::Config) {
        info!("Initializing Mysql db {DB_NAME}...");

        self.rt.spawn(async move {
            let database_url = secure_config.construct_url(DB_NAME);
            let database_url = database_url.unsecure();
            let pool = mysql_async::Pool::new(Opts::from_url(database_url).expect("catch this error"));

            let mut _conn = pool.get_conn().await.expect("catch this error");
        });
    }

    async fn add_answer(&mut self, _user: u64, _id: u16, _answer: DbAnswer) {}
}
