#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}

impl From<DbError> for prono::Error {
    fn from(e: DbError) -> prono::Error {
        prono::Error::Repository(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_error_display() {
        // We can't easily create a sqlx::Error, but we can test the conversion
        // by verifying the trait is implemented correctly
        let db_error = DbError::DatabaseError(sqlx::Error::RowNotFound);
        let display = format!("{db_error}");
        assert!(display.contains("Database error"));
    }

    #[test]
    fn test_db_error_to_prono_error_conversion() {
        let db_error = DbError::DatabaseError(sqlx::Error::RowNotFound);
        let prono_error: prono::Error = db_error.into();
        assert!(matches!(prono_error, prono::Error::Repository(msg) if msg.contains("Database error")));
    }
}
