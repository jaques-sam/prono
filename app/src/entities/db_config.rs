use secure_string::SecureString;
use serde::Deserialize;

#[derive(Deserialize)]
#[cfg_attr(test, derive(Clone))]
pub struct DbConfig {
    host: SecureString,
    #[serde(deserialize_with = "deserialize_as_u16")]
    port: SecureString,
    user: SecureString,
    pass: SecureString,
}

fn deserialize_as_u16<'de, D>(deserializer: D) -> Result<SecureString, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let number: u16 = Deserialize::deserialize(deserializer)?;
    Ok(number.to_string().into())
}

impl DbConfig {
    pub fn apply_overrides(&mut self, overrides: DbConfigOverrides) {
        if let Some(host) = overrides.host {
            self.host = host;
        }
        if let Some(port) = overrides.port {
            self.port = port;
        }
        if let Some(user) = overrides.user {
            self.user = user;
        }
        if let Some(pass) = overrides.pass {
            self.pass = pass;
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<DbConfig> for prono_db::Config {
    fn from(db_config: DbConfig) -> Self {
        Self {
            host: db_config.host,
            port: db_config.port,
            user: db_config.user,
            pass: db_config.pass,
            db_name: crate::DB_NAME.to_string(),
        }
    }
}

#[cfg_attr(test, derive(Clone))]
pub struct DbConfigOverrides {
    pub host: Option<SecureString>,
    pub port: Option<SecureString>,
    pub user: Option<SecureString>,
    pub pass: Option<SecureString>,
}

impl TryFrom<DbConfigOverrides> for DbConfig {
    type Error = &'static str;

    fn try_from(overrides: DbConfigOverrides) -> Result<Self, &'static str> {
        Ok(Self {
            host: overrides.host.ok_or("host override is missing")?,
            port: overrides.port.ok_or("port override is missing")?,
            user: overrides.user.ok_or("user override is missing")?,
            pass: overrides.pass.ok_or("pass override is missing")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use secure_string::SecureString;

    use super::*;

    #[rstest]
    #[case(Some("localhost"), Some("5555"), Some("user"), Some("password"))]
    #[case(None, Some("5555"), Some("user"), Some("password"))]
    #[case(Some("localhost"), None, Some("user"), Some("password"))]
    #[case(Some("localhost"), Some("5555"), None, Some("password"))]
    #[case(Some("localhost"), Some("5555"), Some("user"), None)]
    fn test_config_overrides_are_used_when_constructing_db_config(
        #[case] host_override: Option<&str>,
        #[case] port_override: Option<&str>,
        #[case] user_override: Option<&str>,
        #[case] pass_override: Option<&str>,
    ) {
        let overrides = DbConfigOverrides {
            host: host_override.map(Into::into),
            port: port_override.map(Into::into),
            user: user_override.map(Into::into),
            pass: pass_override.map(Into::into),
        };

        let mut config = DbConfig {
            host: SecureString::from("localhost"),
            port: SecureString::from("5555"),
            user: SecureString::from("user"),
            pass: SecureString::from("password"),
        };

        config.apply_overrides(overrides);

        if host_override.is_some() {
            assert_eq!(config.host.unsecure(), "localhost");
        }
        if port_override.is_some() {
            assert_eq!(config.port.unsecure(), "5555");
        }
        if user_override.is_some() {
            assert_eq!(config.user.unsecure(), "user");
        }
        if pass_override.is_some() {
            assert_eq!(config.pass.unsecure(), "password");
        }
    }

    #[rstest]
    #[case(None, Some("5555"), Some("user"), Some("password"))]
    #[case(Some("localhost"), None, Some("user"), Some("password"))]
    #[case(Some("localhost"), Some("5555"), None, Some("password"))]
    #[case(Some("localhost"), Some("5555"), Some("user"), None)]
    #[should_panic]
    fn test_config_file_not_found_when_constructing_db_config(
        #[case] host_override: Option<&str>,
        #[case] port_override: Option<&str>,
        #[case] user_override: Option<&str>,
        #[case] pass_override: Option<&str>,
    ) {
        generic::add_panic_hook();

        let overrides = DbConfigOverrides {
            host: host_override.map(Into::into),
            port: port_override.map(Into::into),
            user: user_override.map(Into::into),
            pass: pass_override.map(Into::into),
        };

        let mut config: DbConfig = overrides.clone().try_into().unwrap();
        config.apply_overrides(overrides);
    }
}
