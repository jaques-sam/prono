use serde::Deserialize;

const CONFIG_FILENAME: &str = "secure_config.toml";

/// Database connection settings.
///
/// Deliberately not `Debug`: it holds the password.
pub struct DbConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub pass: String,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigFile {
    #[serde(default)]
    db: PartialDb,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct PartialDb {
    host: Option<String>,
    port: Option<u16>,
    user: Option<String>,
    pass: Option<String>,
}

impl DbConfig {
    /// Reads `<config dir>/prono/secure_config.toml` when it exists; each
    /// non-empty `PRONO_DB_HOST`, `PRONO_DB_PORT`, `PRONO_DB_USER` and
    /// `PRONO_DB_PASS` environment variable overrides its setting.
    ///
    /// # Errors
    ///
    /// A readable message if the file is invalid or a setting is missing.
    pub fn load() -> Result<Self, String> {
        let path = dirs::config_dir()
            .unwrap_or_default()
            .join("prono")
            .join(CONFIG_FILENAME);
        let file = match std::fs::read_to_string(&path) {
            Ok(text) => Some(text),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
            Err(err) => return Err(format!("cannot read {}: {err}", path.display())),
        };
        Self::from_sources(file.as_deref(), |key| std::env::var(key).ok())
            .map_err(|err| format!("{err} (config file: {})", path.display()))
    }

    fn from_sources(file: Option<&str>, env: impl Fn(&str) -> Option<String>) -> Result<Self, String> {
        let mut db = match file {
            Some(text) => {
                toml::from_str::<ConfigFile>(text)
                    .map_err(|err| format!("invalid config file: {err}"))?
                    .db
            }
            None => PartialDb::default(),
        };

        let env = |key: &str| env(key).filter(|value| !value.is_empty());
        if let Some(host) = env("PRONO_DB_HOST") {
            db.host = Some(host);
        }
        if let Some(port) = env("PRONO_DB_PORT") {
            db.port = Some(
                port.parse()
                    .map_err(|_| format!("PRONO_DB_PORT is not a port: {port}"))?,
            );
        }
        if let Some(user) = env("PRONO_DB_USER") {
            db.user = Some(user);
        }
        if let Some(pass) = env("PRONO_DB_PASS") {
            db.pass = Some(pass);
        }

        match (db.host, db.port, db.user, db.pass) {
            (Some(host), Some(port), Some(user), Some(pass)) => Ok(Self { host, port, user, pass }),
            (host, port, user, pass) => {
                let missing: Vec<_> = [
                    ("host", host.is_none()),
                    ("port", port.is_none()),
                    ("user", user.is_none()),
                    ("pass", pass.is_none()),
                ]
                .into_iter()
                .filter_map(|(name, missing)| missing.then_some(name))
                .collect();
                Err(format!("missing database setting(s): {}", missing.join(", ")))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = r#"
        [db]
        host = "host"
        port = 5555
        user = "user"
        pass = "pass"
    "#;

    fn no_env(_: &str) -> Option<String> {
        None
    }

    #[test]
    fn reads_file() {
        let config = DbConfig::from_sources(Some(FILE), no_env).unwrap();
        assert_eq!(
            (
                config.host.as_str(),
                config.port,
                config.user.as_str(),
                config.pass.as_str()
            ),
            ("host", 5555, "user", "pass")
        );
    }

    #[test]
    fn env_overrides_file() {
        let env = |key: &str| match key {
            "PRONO_DB_HOST" => Some("other".to_string()),
            "PRONO_DB_PORT" => Some("3306".to_string()),
            "PRONO_DB_USER" => Some(String::new()), // empty: ignored
            _ => None,
        };
        let config = DbConfig::from_sources(Some(FILE), env).unwrap();
        assert_eq!(
            (config.host.as_str(), config.port, config.user.as_str()),
            ("other", 3306, "user")
        );
    }

    #[test]
    fn env_alone_is_enough() {
        let env = |key: &str| Some(if key == "PRONO_DB_PORT" { "1".into() } else { key.into() });
        assert!(DbConfig::from_sources(None, env).is_ok());
    }

    #[test]
    fn missing_settings_are_listed() {
        let err = DbConfig::from_sources(Some("[db]\nhost = \"h\"\nport = 1"), no_env)
            .err()
            .unwrap();
        assert_eq!(err, "missing database setting(s): user, pass");
    }

    #[test]
    fn invalid_input_is_reported() {
        assert!(DbConfig::from_sources(Some("[db]\nport = \"x\""), no_env).is_err());
        let bad_port = |key: &str| (key == "PRONO_DB_PORT").then(|| "x".to_string());
        assert!(DbConfig::from_sources(Some(FILE), bad_port).is_err());
    }
}
