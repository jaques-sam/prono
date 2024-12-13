use std::path::Path;

use log::{debug, info};
// use mysql_async::prelude::*;
use secure_string::SecureString;
use serde::Deserialize;

use crate::{DbAnswer, DB};

static DB_NAME: &str = "db_proto";
static SECURE_CONFIG_FILE: &str = "secure_config.toml";
static HOST_OVERRIDE_ENV_VAR: &str = "DB_HOST_OVERRIDE";
static PORT_OVERRIDE_ENV_VAR: &str = "DB_PORT_OVERRIDE";
static USER_OVERRIDE_ENV_VAR: &str = "DB_USER_OVERRIDE";
static PASS_OVERRIDE_ENV_VAR: &str = "DB_PASS_OVERRIDE";

struct EnvVarOverrides {
    host: Option<SecureString>,
    port: Option<SecureString>,
    user: Option<SecureString>,
    pass: Option<SecureString>,
}

impl EnvVarOverrides {
    fn all_exist(&self) -> bool {
        self.host.is_some() && self.port.is_some() && self.user.is_some() && self.pass.is_some()
    }
}

#[derive(Deserialize)]
#[cfg_attr(test, derive(Clone))]
struct DBConfig {
    host: SecureString,
    #[serde(deserialize_with = "deserialize_as_u16")]
    port: SecureString,
    user: SecureString,
    pass: SecureString,
}

fn deserialize_as_u16<'de, D>(deserializer: D) -> Result<SecureString, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let number: u16 = Deserialize::deserialize(deserializer)?;
    Ok(number.to_string().into())
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
        db.initialize(Path::new(SECURE_CONFIG_FILE));
        db
    }
}

impl DB for MysqlDb {
    fn initialize(&self, config: &Path) {
        let overrides = EnvVarOverrides {
            host: std::env::var(HOST_OVERRIDE_ENV_VAR).ok().map(Into::into),
            port: std::env::var(PORT_OVERRIDE_ENV_VAR).ok().map(Into::into),
            user: std::env::var(USER_OVERRIDE_ENV_VAR).ok().map(Into::into),
            pass: std::env::var(PASS_OVERRIDE_ENV_VAR).ok().map(Into::into),
        };

        let secure_config = std::fs::read_to_string(config).expect("secure config is missing");
        let secure_config: Option<SecureConfig> = toml::from_str(&secure_config).ok();

        let db_config = construct_db_config(overrides, secure_config);

        let _url = construct_url(db_config);
        info!("MysqlDb initialized");
    }

    fn add_answer(&mut self, _user: u64, _id: u16, _answer: DbAnswer) {}
}

fn construct_url(db_config: DBConfig) -> SecureString {
    static DB_PROTOCOL: &str = "mysql";

    let DBConfig { host, port, user, pass } = db_config;

    let host = host.unsecure();
    let port = port.unsecure();
    let user = user.unsecure();
    let pass = pass.unsecure();

    format!("{DB_PROTOCOL}://{user}:{pass}@{host}:{port}/{DB_NAME}").into()
}

fn construct_db_config(overrides: EnvVarOverrides, secure_config: Option<SecureConfig>) -> DBConfig {
    if overrides.all_exist() {
        debug!("All secret environment vars are set");
        DBConfig {
            host: overrides.host.expect("Host must be set"),
            port: overrides.port.expect("Port must be set"),
            user: overrides.user.expect("User must be set"),
            pass: overrides.pass.expect("Pass must be set"),
        }
    } else {
        let secure_config = secure_config.expect("secure config from file");
        debug!("Some or no secret environment vars are set. Read remaining config from secure_config.toml");
        let mut db_config: DBConfig;
        db_config = secure_config.db;

        if let Some(host) = overrides.host {
            db_config.host = host;
        }
        if let Some(port) = overrides.port {
            db_config.port = port;
        }
        if let Some(user) = overrides.user {
            db_config.user = user;
        }
        if let Some(pass) = overrides.pass {
            db_config.pass = pass;
        }
        db_config
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[test]
    fn test_construct_url() {
        let db_config = DBConfig {
            host: SecureString::from("localhost"),
            port: SecureString::from("5555"),
            user: SecureString::from("user"),
            pass: SecureString::from("password"),
        };

        let expected_host = db_config.host.unsecure();
        let expected_user = db_config.user.unsecure();
        let expected_pass = db_config.pass.unsecure();
        let expected_port = db_config.port.unsecure();
        let expected_url = format!("mysql://{expected_user}:{expected_pass}@{expected_host}:{expected_port}/{DB_NAME}");

        let constructed_url = construct_url(db_config);

        assert_eq!(constructed_url.unsecure(), expected_url);
    }

    #[test]
    fn test_initialize_valid_config_file() {
        let mysql_db = MysqlDb {}; // not using new() because we don't want to initialize
        let mut filename = Path::new(file!()).parent().unwrap().to_path_buf();
        filename.push("test_config.toml");
        mysql_db.initialize(&filename);
    }

    #[test]
    #[should_panic]
    fn test_initialize_without_config_file() {
        crate::generic::add_panic_hook();

        let mysql_db = MysqlDb::new();
        mysql_db.initialize(Path::new("file_does_not_exist.toml"));
    }

    #[rstest]
    #[case(Some("localhost"), Some("5555"), Some("user"), Some("password"))]
    #[case(None, Some("5555"), Some("user"), Some("password"))]
    #[case(Some("localhost"), None, Some("user"), Some("password"))]
    #[case(Some("localhost"), Some("5555"), None, Some("password"))]
    #[case(Some("localhost"), Some("5555"), Some("user"), None)]
    fn test_config_overrides_are_used_when_constructing_db_config(
        #[case] host_override: Option<&str>,
        #[case] port_override: Option<&str>,
        #[case] user_override: Option<&str>,
        #[case] pass_override: Option<&str>,
    ) {
        let overrides = EnvVarOverrides {
            host: host_override.map(Into::into),
            port: port_override.map(Into::into),
            user: user_override.map(Into::into),
            pass: pass_override.map(Into::into),
        };

        let db_config = DBConfig {
            host: SecureString::from("localhost"),
            port: SecureString::from("5555"),
            user: SecureString::from("user"),
            pass: SecureString::from("password"),
        };
        let expected_full_db_config = db_config.clone();

        construct_db_config(overrides, Some(SecureConfig { db: db_config }));

        if host_override.is_some() {
            assert_eq!(expected_full_db_config.host.unsecure(), "localhost");
        }
        if port_override.is_some() {
            assert_eq!(expected_full_db_config.port.unsecure(), "5555");
        }
        if user_override.is_some() {
            assert_eq!(expected_full_db_config.user.unsecure(), "user");
        }
        if pass_override.is_some() {
            assert_eq!(expected_full_db_config.pass.unsecure(), "password");
        }
    }

    #[rstest]
    #[case(None, Some("5555"), Some("user"), Some("password"))]
    #[case(Some("localhost"), None, Some("user"), Some("password"))]
    #[case(Some("localhost"), Some("5555"), None, Some("password"))]
    #[case(Some("localhost"), Some("5555"), Some("user"), None)]
    #[should_panic]
    fn test_config_file_not_found_when_constructing_db_config(
        #[case] host_override: Option<&str>,
        #[case] port_override: Option<&str>,
        #[case] user_override: Option<&str>,
        #[case] pass_override: Option<&str>,
    ) {
        crate::generic::add_panic_hook();

        let overrides = EnvVarOverrides {
            host: host_override.map(Into::into),
            port: port_override.map(Into::into),
            user: user_override.map(Into::into),
            pass: pass_override.map(Into::into),
        };

        construct_db_config(overrides, None);
    }
}
