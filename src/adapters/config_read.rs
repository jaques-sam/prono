use std::path::Path;

use log::debug;
// use mysql_async::prelude::*;

use crate::{proto_db, ReadConfig, SecureConfig};

static HOST_OVERRIDE_ENV_VAR: &str = "DB_HOST_OVERRIDE";
static PORT_OVERRIDE_ENV_VAR: &str = "DB_PORT_OVERRIDE";
static USER_OVERRIDE_ENV_VAR: &str = "DB_USER_OVERRIDE";
static PASS_OVERRIDE_ENV_VAR: &str = "DB_PASS_OVERRIDE";

#[derive(Default)]
pub struct ConfigRead {}

impl ReadConfig<SecureConfig> for ConfigRead {
    fn read(&self, config: &Path) -> SecureConfig {
        let overrides = proto_db::ConfigOverrides {
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
            SecureConfig::new(
                overrides
                    .try_into()
                    .expect("expect all overrides are set through env vars"),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rstest::rstest;

    #[test]
    fn test_read_valid_config_file() {
        let reader = ConfigRead {};
        let mut filename = Path::new(file!()).parent().unwrap().to_path_buf();
        filename.push("test_config.toml");
        reader.read(&filename);
    }

    #[test]
    #[should_panic]
    fn test_read_without_config_file_fails() {
        crate::generic::add_panic_hook();

        let reader = ConfigRead {};
        reader.read(Path::new("file_does_not_exist.toml"));
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
        use proto_db::{Config, ConfigOverrides};

        crate::generic::add_panic_hook();

        let overrides = ConfigOverrides {
            host: host_override.map(Into::into),
            port: port_override.map(Into::into),
            user: user_override.map(Into::into),
            pass: pass_override.map(Into::into),
        };

        let mut config: Config = overrides.clone().try_into().unwrap();
        config.apply_overrides(overrides);
    }
}
