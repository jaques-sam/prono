use std::{fs, path};

use crate::ReadConfig;
use log::{debug, info, warn};

use crate::SecureConfig;

static HOST_OVERRIDE_ENV_VAR: &str = "PRONO_DB_HOST";
static PORT_OVERRIDE_ENV_VAR: &str = "PRONO_DB_PORT";
static USER_OVERRIDE_ENV_VAR: &str = "PRONO_DB_USER";
static PASS_OVERRIDE_ENV_VAR: &str = "PRONO_DB_PASS";
static CONFIG_FILENAME: &str = "secure_config.toml";

#[derive(Default)]
pub struct ConfigReader {}

impl ReadConfig<SecureConfig> for ConfigReader {
    fn default_config_path(&self) -> path::PathBuf {
        let path = dirs::config_dir().unwrap().join("prono").join(CONFIG_FILENAME);
        debug!("Default config path: {}", path.display());
        path
    }

    fn read<P: AsRef<path::Path>>(&self, config: P) -> SecureConfig {
        let non_empty = |s: String| if s.is_empty() { None } else { Some(s.into()) };
        let overrides = crate::db_config::Overrides {
            host: std::env::var(HOST_OVERRIDE_ENV_VAR).ok().and_then(non_empty),
            port: std::env::var(PORT_OVERRIDE_ENV_VAR).ok().and_then(non_empty),
            user: std::env::var(USER_OVERRIDE_ENV_VAR).ok().and_then(non_empty),
            pass: std::env::var(PASS_OVERRIDE_ENV_VAR).ok().and_then(non_empty),
        };

        let config_path = config.as_ref();
        let file_content = fs::read_to_string(config_path).ok();

        if file_content.is_none() {
            info!(
                "No config file found at {}, falling back to environment variables",
                config_path.display()
            );
        }

        let parsed = file_content.as_deref().and_then(|s| {
            toml::from_str::<SecureConfig>(s)
                .map_err(|e| warn!("Failed to parse secure config: {e}"))
                .ok()
        });

        if let Some(secure_config) = parsed {
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
        ConfigReader {}.read(&filename);
    }

    #[test]
    #[should_panic(expected = "expect all overrides are set through env vars")]
    fn test_read_without_config_file_fails() {
        generic::add_panic_hook();

        ConfigReader {}.read(Path::new("file_does_not_exist.toml"));
    }
}
