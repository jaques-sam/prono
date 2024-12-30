use secure_string::SecureString;

pub struct Config {
    pub host: SecureString,
    pub port: SecureString,
    pub user: SecureString,
    pub pass: SecureString,
    pub db_name: String,
}

impl Config {
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
}
