use crate::{DbConfig, DbConfigOverrides};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct SecureConfig {
    #[serde(rename = "db")]
    pub db: DbConfig,
}

impl SecureConfig {
    pub fn override_db_config(mut self, overrides: DbConfigOverrides) -> Self {
        self.db.apply_overrides(overrides);
        self
    }
}
