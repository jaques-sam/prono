use std::path::Path;

use log::debug;
use prono::ReadConfig;

use crate::SecureConfig;
// use mysql_async::prelude::*;

static HOST_OVERRIDE_ENV_VAR: &str = "DB_HOST_OVERRIDE";
static PORT_OVERRIDE_ENV_VAR: &str = "DB_PORT_OVERRIDE";
static USER_OVERRIDE_ENV_VAR: &str = "DB_USER_OVERRIDE";
static PASS_OVERRIDE_ENV_VAR: &str = "DB_PASS_OVERRIDE";

#[derive(Default)]
pub struct ConfigRead {}

impl ReadConfig<SecureConfig> for ConfigRead {
    fn read(&self, config: &Path) -> SecureConfig {
        let overrides = crate::db_config::Overrides {
            host: std::env::var(HOST_OVERRIDE_ENV_VAR).ok().map(Into::into),
            port: std::env::var(PORT_OVERRIDE_ENV_VAR).ok().map(Into::into),
            user: std::env::var(USER_OVERRIDE_ENV_VAR).ok().map(Into::into),
            pass: std::env::var(PASS_OVERRIDE_ENV_VAR).ok().map(Into::into),
        };

        let secure_config = std::fs::read_to_string(config).expect("secure config is missing");
        let secure_config: Option<SecureConfig> = toml::from_str(&secure_config).ok();

        if let Some(secure_config) = secure_config {
            debug!("Some or no secret environment vars are set. Read remaining config from secure_config.toml");
            secure_config.override_db_config(overrides)
        } else {
            debug!("All secret environment vars are set");
            SecureConfig {
                db: overrides
                    .try_into()
                    .expect("expect all overrides are set through env vars"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_valid_config_file() {
        let reader = ConfigRead {};
        let filename = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
            .join(file!())
            .parent()
            .unwrap()
            .join("test_config.toml");
        reader.read(&filename);
    }

    #[test]
    #[should_panic(expected = "secure config is missing")]
    fn test_read_without_config_file_fails() {
        generic::add_panic_hook();

        let reader = ConfigRead {};
        reader.read(Path::new("file_does_not_exist.toml"));
    }
}
