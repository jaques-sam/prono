use crate::db_config;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct SecureConfig {
    #[serde(rename = "db")]
    pub db: db_config::Config,
}

impl SecureConfig {
    pub fn override_db_config(mut self, overrides: db_config::Overrides) -> Self {
        self.db.apply_overrides(overrides);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secure_string::SecureString;

    fn create_test_config() -> SecureConfig {
        SecureConfig {
            db: db_config::Config::try_from(db_config::Overrides {
                host: Some(SecureString::from("localhost")),
                port: Some(SecureString::from("3306")),
                user: Some(SecureString::from("testuser")),
                pass: Some(SecureString::from("testpass")),
            })
            .unwrap(),
        }
    }

    #[test]
    fn test_override_db_config_with_host() {
        let config = create_test_config();
        let overrides = db_config::Overrides {
            host: Some(SecureString::from("newhost")),
            ..db_config::Overrides::default()
        };

        let result = config.override_db_config(overrides);
        let db_config: prono_db::Config = result.db.into();
        assert_eq!(db_config.host.unsecure(), "newhost");
    }

    #[test]
    fn test_override_db_config_with_no_overrides() {
        let config = create_test_config();
        let overrides = db_config::Overrides::default();

        let result = config.override_db_config(overrides);
        let db_config: prono_db::Config = result.db.into();
        assert_eq!(db_config.host.unsecure(), "localhost");
    }

    #[test]
    fn test_override_db_config_returns_self() {
        let config = create_test_config();
        let overrides = db_config::Overrides {
            host: Some(SecureString::from("chained")),
            ..db_config::Overrides::default()
        };

        // Test that the method returns self for chaining
        let _result = config.override_db_config(overrides);
    }
}
