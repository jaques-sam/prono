-- Tables of the prono database. Idempotent; run inside `db_prono`.

CREATE TABLE IF NOT EXISTS Users (
    user_id BIGINT AUTO_INCREMENT PRIMARY KEY,
    user_name TEXT NOT NULL,
    email TEXT,
    device_id VARCHAR(255) NOT NULL UNIQUE,
    INDEX idx_user_name (user_name(255))
);

-- One answer per user and question; the unique key also serves lookups by user.
CREATE TABLE IF NOT EXISTS AnswerResponse (
    user_id BIGINT NOT NULL,
    -- A UUID, stored as its 16 bytes (`sqlx` does this for `uuid::Uuid`).
    question_id BINARY(16) NOT NULL,
    answer TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES Users(user_id) ON DELETE CASCADE,
    INDEX idx_question (question_id),
    UNIQUE KEY uq_user_question (user_id, question_id)
);
