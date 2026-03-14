use secure_string::SecureString;

pub static DB_NAME: &str = "db_prono";

pub struct Config {
    pub host: SecureString,
    pub port: SecureString,
    pub user: SecureString,
    pub pass: SecureString,
    pub db_name: String,
}

impl Config {
    #[must_use]
    pub fn construct_url(&self) -> SecureString {
        static DB_PROTOCOL: &str = "mysql";

        let host = self.host.unsecure();
        let port = self.port.unsecure();
        let user = self.user.unsecure();
        let pass = self.pass.unsecure();
        let db_name = &self.db_name;

        format!("{DB_PROTOCOL}://{user}:{pass}@{host}:{port}/{db_name}").into()
    }
}

impl From<prono::db_config::Config> for Config {
    fn from(db_config: prono::db_config::Config) -> Self {
        Self {
            host: db_config.host,
            port: db_config.port,
            user: db_config.user,
            pass: db_config.pass,
            db_name: DB_NAME.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use secure_string::SecureString;

    use super::*;

    #[test]
    fn test_construct_url() {
        let db_config = Config {
            host: SecureString::from("localhost"),
            port: SecureString::from("5555"),
            user: SecureString::from("user"),
            pass: SecureString::from("password"),
            db_name: "db_name".to_string(),
        };

        let expected_host = db_config.host.unsecure();
        let expected_user = db_config.user.unsecure();
        let expected_pass = db_config.pass.unsecure();
        let expected_port = db_config.port.unsecure();
        let expected_url = format!("mysql://{expected_user}:{expected_pass}@{expected_host}:{expected_port}/db_name");

        let constructed_url = db_config.construct_url();

        assert_eq!(constructed_url.unsecure(), expected_url);
    }

    #[test]
    fn test_from_prono_db_config() {
        let prono_config = prono::db_config::Config {
            host: SecureString::from("myhost"),
            port: SecureString::from("3306"),
            user: SecureString::from("root"),
            pass: SecureString::from("secret"),
        };

        let config: Config = prono_config.into();

        assert_eq!(config.host.unsecure(), "myhost");
        assert_eq!(config.port.unsecure(), "3306");
        assert_eq!(config.user.unsecure(), "root");
        assert_eq!(config.pass.unsecure(), "secret");
        assert_eq!(config.db_name, DB_NAME);
    }
}
