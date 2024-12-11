// use mysql_async::prelude::*;
use secure_string::SecureString;
use serde::Deserialize;

use crate::{DbAnswer, DB};

#[derive(Deserialize)]
struct DBConfig {
    host: String,
    port: u16,
    user: SecureString,
    pass: SecureString,
    name: String,
}

#[derive(Deserialize)]
struct SecureConfig {
    #[serde(rename = "db")]
    db: DBConfig,
}

#[derive(Default)]
pub struct MysqlDb {}

impl MysqlDb {
    pub fn new() -> Self {
        let db = Self::default();
        db.initialize();
        db
    }
}

impl DB for MysqlDb {
    fn initialize(&self) {
        // TODO ==> use ENV variables to fill in sensitive data (e.g. put in github).
        // This can be chosen based on valid UTF-8 file content.
        let secure_config = include_str!("../../secure_config.toml");
        let secure_config: SecureConfig = toml::from_str(secure_config).expect("secure_config.toml cannot be parsed");

        let db_url: SecureString = construct_url(secure_config.db).into();
        println!("MysqlDb initialized with URL: {}", db_url.unsecure());
    }

    fn add_answer(&mut self, _user: u64, _id: u16, _answer: DbAnswer) {}
}

fn construct_url(db_config: DBConfig) -> String {
    static DB_PROTOCOL: &str = "mysql";

    let DBConfig {
        host,
        port,
        user,
        pass,
        name,
    } = db_config;

    let user = user.unsecure();
    let pass = pass.unsecure();

    format!("{DB_PROTOCOL}://{user}:{pass}@{host}:{port}/{name}")
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_construct_url() {
        let db_config = DBConfig {
            host: "localhost".to_string(),
            port: 3306,
            user: SecureString::from("user"),
            pass: SecureString::from("password"),
            name: "test_db".to_string(),
        };

        let expected_user = db_config.user.unsecure();
        let expected_pass = db_config.pass.unsecure();
        let expected_url = format!("mysql://{expected_user}:{expected_pass}@localhost:3306/test_db");

        let constructed_url = construct_url(db_config);

        assert_eq!(constructed_url, expected_url);
    }

    #[test]
    fn test_initialize() {
        let mysql_db = MysqlDb::new();
        let output = std::panic::catch_unwind(|| {
            mysql_db.initialize();
        });

        assert!(output.is_ok());
    }
}

