//! `MysqlRepo` against a real MariaDB/MySQL with `backend/install/schema.sql`
//! applied. Ignored by default; point it at a **dev** database:
//!
//! ```sh
//! PRONO_DB_HOST=127.0.0.1 PRONO_DB_PORT=13306 PRONO_DB_USER=prono-app PRONO_DB_PASS=devpass \
//!     cargo test -p prono --test mysql -- --ignored
//! ```

use prono::{DbConfig, Error, MysqlRepo, Repo};
use prono_api::{Answer, AnswerKind, Question, UserAnswer};
use uuid::Uuid;

fn config() -> DbConfig {
    let var = |key: &str| std::env::var(key).unwrap_or_else(|_| panic!("set {key} to a dev database"));
    DbConfig {
        host: var("PRONO_DB_HOST"),
        port: var("PRONO_DB_PORT").parse().expect("a port number"),
        user: var("PRONO_DB_USER"),
        pass: var("PRONO_DB_PASS"),
    }
}

/// A fresh id per run, so runs never collide in a shared dev database.
fn unique(what: &str) -> Uuid {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    Uuid::new_v5(&Uuid::NAMESPACE_DNS, format!("{what}{nanos}").as_bytes())
}

/// A text question with a fresh id.
fn text_question(what: &str) -> Question {
    Question {
        id: unique(what),
        text: what.into(),
        kind: AnswerKind::Text,
        possible_answers: Vec::new(),
        other_label: None,
        correct_answer: None,
    }
}

#[tokio::test]
#[ignore = "needs a dev database, see the module docs"]
async fn answers_round_trip_by_uuid_and_duplicates_are_refused() {
    let repo = MysqlRepo::connect(&config()).await.unwrap();
    let user = format!("it-{}", unique("user"));
    let device = format!("dev-{}", unique("device"));
    let (question1, question2) = (text_question("q1"), text_question("q2"));
    let (q1, q2) = (question1.id, question2.id);
    let text = |s: &str| Answer::Text(s.into());

    repo.save_answers(&user, &device, &[(q1, text("a"))]).await.unwrap();
    assert_eq!(repo.device_of(&user).await.unwrap().as_deref(), Some(device.as_str()));
    assert_eq!(
        repo.answers(&question1).await.unwrap(),
        [UserAnswer {
            user: user.clone(),
            answer: text("a")
        }]
    );
    assert_eq!(repo.answers(&question2).await.unwrap(), Vec::<UserAnswer>::new());

    // The same question again, in a batch with a new one: all or nothing.
    let err = repo
        .save_answers(&user, &device, &[(q2, text("b")), (q1, text("c"))])
        .await
        .unwrap_err();
    assert_eq!(err, Error::AnswerExists);
    assert_eq!(repo.answers(&question2).await.unwrap(), Vec::<UserAnswer>::new());

    repo.delete_user(&user).await.unwrap();
    assert_eq!(repo.answers(&question1).await.unwrap(), Vec::<UserAnswer>::new());
}

#[tokio::test]
#[ignore = "needs a dev database, see the module docs"]
async fn pass_phrases_are_stored_and_replaced_by_survey() {
    let repo = MysqlRepo::connect(&config()).await.unwrap();
    // A survey id no real survey uses, so runs never touch real pass phrases.
    let survey_id = u64::MAX - 1;
    let phrase_of = async |repo: &MysqlRepo| {
        let all = repo.pass_phrases().await.unwrap();
        all.into_iter()
            .find(|(id, _)| *id == survey_id)
            .map(|(_, phrase)| phrase)
    };

    assert_eq!(phrase_of(&repo).await, None);
    repo.set_pass_phrase(survey_id, "one").await.unwrap();
    repo.set_pass_phrase(survey_id, "two").await.unwrap();
    assert_eq!(phrase_of(&repo).await.as_deref(), Some("two"));
}

#[tokio::test]
#[ignore = "needs a dev database, see the module docs"]
async fn survey_definitions_are_stored_and_replaced_by_id() {
    let repo = MysqlRepo::connect(&config()).await.unwrap();
    // A survey id no real survey uses, so runs never touch real surveys.
    let survey_id = u64::MAX - 2;

    repo.save_survey_definition(survey_id, "first").await.unwrap();
    repo.save_survey_definition(survey_id, "second").await.unwrap();
    assert_eq!(
        repo.survey_definition(survey_id).await.unwrap().as_deref(),
        Some("second")
    );
    assert!(repo.survey_definitions().await.unwrap().contains(&"second".to_string()));
    assert_eq!(repo.survey_definition(survey_id - 1).await.unwrap(), None);
}
