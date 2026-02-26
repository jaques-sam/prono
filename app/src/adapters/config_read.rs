use std::{fs, path};

use log::{debug, error, info};
use prono::ReadConfig;

use crate::SecureConfig;

static HOST_OVERRIDE_ENV_VAR: &str = "PRONO_DB_HOST";
static PORT_OVERRIDE_ENV_VAR: &str = "PRONO_DB_PORT";
static USER_OVERRIDE_ENV_VAR: &str = "PRONO_DB_USER";
static PASS_OVERRIDE_ENV_VAR: &str = "PRONO_DB_PASS";
static CONFIG_FILENAME: &str = "secure_config.toml";

#[derive(Default)]
pub struct ConfigRead {}

impl ReadConfig<SecureConfig> for ConfigRead {
    fn default_config_path() -> path::PathBuf {
        let path = dirs::config_dir().unwrap().join("prono").join(CONFIG_FILENAME);
        debug!("Default config path: {}", path.display());
        path
    }

    fn read<P: AsRef<path::Path>>(config: P) -> SecureConfig {
        let overrides = crate::db_config::Overrides {
            host: std::env::var(HOST_OVERRIDE_ENV_VAR).ok().map(Into::into),
            port: std::env::var(PORT_OVERRIDE_ENV_VAR).ok().map(Into::into),
            user: std::env::var(USER_OVERRIDE_ENV_VAR).ok().map(Into::into),
            pass: std::env::var(PASS_OVERRIDE_ENV_VAR).ok().map(Into::into),
        };

        let secure_config = fs::read_to_string(config).expect("secure config is missing");
        let secure_config: Option<SecureConfig> = toml::from_str(&secure_config)
            .map_err(|e| {
                error!("Failed to parse secure config: {e}");
                e
            })
            .ok();

        if let Some(secure_config) = secure_config {
            info!("Some or no secret environment vars are set. Read remaining config from secure_config.toml");
            secure_config.override_db_config(overrides)
        } else {
            info!("No/invalid secure config file, read secret environment vars...");
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

    use std::path::Path;

    #[test]
    fn test_read_valid_config_file() {
        let filename = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
            .join(file!())
            .parent()
            .unwrap()
            .join("test_config.toml");
        ConfigRead::read(&filename);
    }

    #[test]
    #[should_panic(expected = "secure config is missing")]
    fn test_read_without_config_file_fails() {
        generic::add_panic_hook();

        ConfigRead::read(Path::new("file_does_not_exist.toml"));
    }
}
