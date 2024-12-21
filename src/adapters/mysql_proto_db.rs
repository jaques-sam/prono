use std::path::Path;

use log::info;
// use mysql_async::prelude::*;

use crate::{
    proto_db::{DbAnswer, DB},
    ReadConfig, SecureConfig,
};

static DB_NAME: &str = "db_proto";
static CONFIG_FILENAME: &str = "secure_config.toml";

#[derive(Default)]
pub struct MysqlDb {}

impl MysqlDb {
    pub fn new(secure_config_read: impl ReadConfig<SecureConfig>) -> Self {
        let db = Self::default();
        let secure_config = secure_config_read.read(Path::new(CONFIG_FILENAME));
        db.initialize(secure_config);
        db
    }
}

impl DB for MysqlDb {
    fn initialize(&self, _secure_config: SecureConfig) {
        info!("Initializing Mysql db {DB_NAME}...");
    }

    fn add_answer(&mut self, _user: u64, _id: u16, _answer: DbAnswer) {}
}
