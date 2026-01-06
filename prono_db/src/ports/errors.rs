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
