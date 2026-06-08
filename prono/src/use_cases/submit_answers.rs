use log::{error, info};

#[cfg(not(debug_assertions))]
use crate::ports::Error;
use crate::ports::PronoResult;
use crate::ports::repo::{self, Answer};

/// Submit a batch of answers for a user.
///
/// Performs these operations consecutively on the repository:
///
/// 1. (release builds only) verify the device matches the registered one,
///    failing fast with [`Error::DeviceMismatch`] if a different device is
///    trying to act as the same user;
/// 2. register / refresh the user's device id (once per batch);
/// 3. store each answer in the batch.
///
/// In debug builds the device check is skipped so the development "Survey
/// again" flow on a single machine isn't blocked.
///
/// # Errors
///
/// Returns on the first error encountered. The user will be registered even
/// if a later answer fails.
pub async fn submit_answers<U, S>(
    users: &U,
    surveys: &S,
    user: &str,
    device_id: &str,
    answers: Vec<(String, Answer)>,
) -> PronoResult<()>
where
    U: repo::Users + ?Sized,
    S: repo::Surveys + ?Sized,
{
    #[cfg(not(debug_assertions))]
    if !users.verify_device(user, device_id).await? {
        error!("Device mismatch for user='{user}' device_id='{device_id}'");
        return Err(Error::DeviceMismatch);
    }

    if let Err(e) = users.add_user(user, device_id).await {
        error!("submit_answers: add_user failed for user='{user}': {e}");
        return Err(e);
    }

    for (question_id, answer) in answers {
        if let Err(e) = surveys.add_answer(user, question_id.clone(), answer).await {
            error!("submit_answers: add_answer failed for user='{user}' Q={question_id}: {e}");
            return Err(e);
        }
        info!("submit_answers: stored answer for user='{user}' Q={question_id}");
    }

    Ok(())
}

#[cfg(all(test, debug_assertions))]
mod tests {
    use super::*;
    use crate::adapters::fake_db::FakeRepo;
    use crate::ports::Error;
    use crate::ports::repo::Db as _;

    #[tokio::test]
    async fn registers_user_and_stores_answers() {
        let repo = FakeRepo::init(()).await.unwrap();
        let answer1 = Answer::Text("hello".into());
        let answer2 = Answer::Text("world".into());

        submit_answers(
            &repo,
            &repo,
            "alice",
            "dev-1",
            vec![("q1".into(), answer1.clone()), ("q2".into(), answer2.clone())],
        )
        .await
        .unwrap();

        let stored1 = repo::Surveys::answer(&repo, "alice", "q1".into()).await;
        assert_eq!(stored1, Some(answer1));
        let stored2 = repo::Surveys::answer(&repo, "alice", "q2".into()).await;
        assert_eq!(stored2, Some(answer2));
    }

    #[tokio::test]
    async fn second_answer_for_same_question_fails() {
        let repo = FakeRepo::init(()).await.unwrap();
        let q = "q1".to_string();

        submit_answers(
            &repo,
            &repo,
            "alice",
            "dev-1",
            vec![(q.clone(), Answer::Text("a".into()))],
        )
        .await
        .unwrap();

        let err = submit_answers(&repo, &repo, "alice", "dev-1", vec![(q, Answer::Text("b".into()))])
            .await
            .unwrap_err();
        assert_eq!(err, Error::AnswerExists);
    }

    #[tokio::test]
    async fn batch_stops_on_first_error() {
        let repo = FakeRepo::init(()).await.unwrap();

        // First submission succeeds
        submit_answers(
            &repo,
            &repo,
            "alice",
            "dev-1",
            vec![("q1".into(), Answer::Text("a".into()))],
        )
        .await
        .unwrap();

        // Second batch includes duplicate q1, should fail before q2 is stored
        let err = submit_answers(
            &repo,
            &repo,
            "alice",
            "dev-1",
            vec![
                ("q1".into(), Answer::Text("b".into())),
                ("q2".into(), Answer::Text("c".into())),
            ],
        )
        .await
        .unwrap_err();
        assert_eq!(err, Error::AnswerExists);

        // q2 should NOT have been stored
        let stored = repo::Surveys::answer(&repo, "alice", "q2".into()).await;
        assert_eq!(stored, None);
    }
}
