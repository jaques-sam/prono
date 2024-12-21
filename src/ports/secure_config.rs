use serde::Deserialize;

use super::proto_db::{Config, ConfigOverrides};

#[derive(Deserialize)]
pub struct SecureConfig {
    #[serde(rename = "db")]
    db: Config,
}

impl SecureConfig {
    pub fn new(db: Config) -> Self {
        Self { db }
    }

    pub fn override_db_config(mut self, overrides: ConfigOverrides) -> Self {
        self.db.apply_overrides(overrides);
        self
    }
}
