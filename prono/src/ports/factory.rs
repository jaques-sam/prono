use crate::{ConfigReader, ReadConfig, SecureConfig};

#[must_use]
pub fn create_config_reader() -> impl ReadConfig<SecureConfig> {
    ConfigReader {}
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_can_create_config_reader() {
        let _config_reader = super::create_config_reader();
    }
}
