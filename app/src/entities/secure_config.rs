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
