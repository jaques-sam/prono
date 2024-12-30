use crate::{DbConfig, DbConfigOverrides};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct SecureConfig {
    #[serde(rename = "db")]
    db: DbConfig,
}

impl SecureConfig {
    pub fn new(db: DbConfig) -> Self {
        Self { db }
    }

    pub fn db(self) -> DbConfig {
        self.db
    }

    pub fn override_db_config(mut self, overrides: DbConfigOverrides) -> Self {
        self.db.apply_overrides(overrides);
        self
    }
}
