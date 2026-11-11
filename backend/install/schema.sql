-- Tables of the prono database. Idempotent; run inside `db_prono`.

-- Everything is utf8mb4: answers, names and surveys contain emoji, which the
-- 3-byte `utf8` (`utf8mb3`) of older databases cannot store.
ALTER DATABASE CHARACTER SET utf8mb4 COLLATE utf8mb4_general_ci;

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

-- The surveys: their JSON definitions, see `api/src/definition.rs`. Uploaded
-- with `prono-cli surveys upload`; there are no surveys until one is.
CREATE TABLE IF NOT EXISTS Survey (
    survey_id BIGINT UNSIGNED PRIMARY KEY,
    definition MEDIUMTEXT NOT NULL
);

-- The pass phrase of a survey: the only way to see and answer it, several
-- surveys may share one. Set with `prono-cli surveys pass-phrase`; the survey
-- stays closed until it is set.
CREATE TABLE IF NOT EXISTS SurveyPassPhrase (
    survey_id BIGINT UNSIGNED PRIMARY KEY,
    pass_phrase VARCHAR(255) NOT NULL
);

-- Existing tables keep the character set they were created with: convert them.
ALTER TABLE Users CONVERT TO CHARACTER SET utf8mb4 COLLATE utf8mb4_general_ci;
ALTER TABLE AnswerResponse CONVERT TO CHARACTER SET utf8mb4 COLLATE utf8mb4_general_ci;
ALTER TABLE Survey CONVERT TO CHARACTER SET utf8mb4 COLLATE utf8mb4_general_ci;
ALTER TABLE SurveyPassPhrase CONVERT TO CHARACTER SET utf8mb4 COLLATE utf8mb4_general_ci;

-- Surveys used to have an `"access"` key (a pass phrase or a name list). Every
-- survey has a pass phrase now and the key is no longer valid in a definition:
-- strip it from the stored ones. Run this before (or while) deploying the
-- backend that no longer reads it. The old name lists (table
-- `SurveyAllowedUser`) are not used any more and are left alone: drop the table
-- when you no longer need them. Surveys that had a name list have no pass
-- phrase: set one with `prono-cli surveys pass-phrase`.
UPDATE Survey SET definition = JSON_REMOVE(definition, '$.access')
    WHERE JSON_CONTAINS_PATH(definition, 'one', '$.access');
