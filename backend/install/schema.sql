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

-- Who may answer a survey with `"access": "name_list"`. Filled with
-- `prono-cli access allow`; a participant matches when `first_name`, a space
-- and `last_name` equal their name, ignoring case.
CREATE TABLE IF NOT EXISTS SurveyAllowedUser (
    survey_id BIGINT UNSIGNED NOT NULL,
    first_name VARCHAR(255) NOT NULL,
    last_name VARCHAR(255) NOT NULL,
    PRIMARY KEY (survey_id, first_name, last_name)
);

-- The pass phrase of a survey with `"access": "pass_phrase"`. Set with
-- `prono-cli access pass-phrase`; the survey stays closed until it is set.
CREATE TABLE IF NOT EXISTS SurveyPassPhrase (
    survey_id BIGINT UNSIGNED PRIMARY KEY,
    pass_phrase VARCHAR(255) NOT NULL
);
