#[derive(Debug, thiserror::Error)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub enum Error {
    #[error("Repository error: {0}")]
    Repository(String),

    #[error("Answer already exists")]
    AnswerExists,

    #[error("Device mismatch: username is registered to a different device")]
    DeviceMismatch,
}

pub type PronoResult<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repository_error_display() {
        let error = Error::Repository("connection failed".to_string());
        assert_eq!(format!("{error}"), "Repository error: connection failed");
    }

    #[test]
    fn test_answer_exists_error_display() {
        let error = Error::AnswerExists;
        assert_eq!(format!("{error}"), "Answer already exists");
    }

    #[test]
    fn test_device_mismatch_error_display() {
        let error = Error::DeviceMismatch;
        assert!(format!("{error}").contains("Device mismatch"));
    }

    #[test]
    fn test_error_equality() {
        assert_eq!(Error::AnswerExists, Error::AnswerExists);
        assert_eq!(Error::DeviceMismatch, Error::DeviceMismatch);
        assert_eq!(
            Error::Repository("test".to_string()),
            Error::Repository("test".to_string())
        );
        assert_ne!(Error::Repository("a".to_string()), Error::Repository("b".to_string()));
    }
}
