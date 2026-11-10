use log::info;
use prono_api::{Access, Invalid, Results, Submission, Survey, UserStatus, check_user_name};

use crate::{Error, Repo, Result};

/// Longest accepted device id (the database column is `VARCHAR(255)`).
const MAX_DEVICE_ID_LEN: usize = 255;

/// Stores `user`'s answers to survey `survey_id`, submitted from `device_id`.
///
/// A name stays bound to the device that first used it, and a device can
/// only be used by one participant.
///
/// # Errors
///
/// [`Error::Invalid`] for a bad submission, [`Error::InvalidDeviceId`],
/// [`Error::NotListed`] or [`Error::WrongPassPhrase`] when the survey's
/// [`Access`] is not met, [`Error::DeviceMismatch`], [`Error::DeviceInUse`],
/// [`Error::AnswerExists`] when answered before, or a repository error.
pub async fn submit(repo: &dyn Repo, survey_id: u64, device_id: &str, submission: &Submission) -> Result<()> {
    let survey = find_survey(survey_id)?;
    submission.validate(survey)?;
    check_device_id(device_id)?;
    check_access(repo, survey, submission).await?;

    let user = submission.user.as_str();
    if repo.device_of(user).await?.is_some_and(|device| device != device_id) {
        return Err(Error::DeviceMismatch);
    }
    if repo.user_of(device_id).await?.is_some_and(|owner| owner != user) {
        return Err(Error::DeviceInUse);
    }

    repo.save_answers(user, device_id, &submission.answers).await?;
    info!(
        "Stored {} answers of '{user}' to survey {survey_id}",
        submission.answers.len()
    );
    Ok(())
}

/// Whether `user` already filled in a survey, and which surveys they may
/// answer, so the app can warn while the name is typed and only offer those
/// surveys. Nothing else about the participants leaves the server.
///
/// # Errors
///
/// [`Error::Invalid`] for a bad name, [`Error::InvalidDeviceId`], or a
/// repository error.
pub async fn user_status(repo: &dyn Repo, device_id: &str, user: &str) -> Result<UserStatus> {
    check_user_name(user)?;
    check_device_id(device_id)?;
    let listed = repo.listed_surveys(user).await?;
    Ok(UserStatus {
        answered: repo.device_of(user).await?.is_some(),
        surveys: prono_api::surveys()
            .iter()
            .filter(|survey| survey.access != Access::NameList || listed.contains(&survey.id))
            .map(|survey| survey.id)
            .collect(),
    })
}

/// Every participant's answers to survey `survey_id`.
///
/// # Errors
///
/// [`Error::Invalid`] for an unknown survey, or a repository error.
pub async fn results(repo: &dyn Repo, survey_id: u64) -> Result<Results> {
    let survey = find_survey(survey_id)?;
    let mut results = Results::new();
    for question in &survey.questions {
        results.insert(question.id, repo.answers(question).await?);
    }
    Ok(results)
}

/// Checks that the participant may answer `survey`. A survey with a pass
/// phrase nobody set yet stays closed.
async fn check_access(repo: &dyn Repo, survey: &Survey, submission: &Submission) -> Result<()> {
    let allowed = match survey.access {
        Access::Open => true,
        Access::NameList => repo.listed_surveys(&submission.user).await?.contains(&survey.id),
        Access::PassPhrase => {
            let stored = repo.pass_phrase(survey.id).await?;
            stored.is_some() && stored == submission.pass_phrase
        }
    };
    match (allowed, survey.access) {
        (true, _) => Ok(()),
        (false, Access::PassPhrase) => Err(Error::WrongPassPhrase),
        (false, _) => Err(Error::NotListed),
    }
}

fn check_device_id(device_id: &str) -> Result<()> {
    if device_id.trim().is_empty() || device_id.len() > MAX_DEVICE_ID_LEN {
        return Err(Error::InvalidDeviceId);
    }
    Ok(())
}

fn find_survey(id: u64) -> Result<&'static Survey> {
    prono_api::survey(id).ok_or(Error::Invalid(Invalid::UnknownSurvey(id)))
}

#[cfg(test)]
mod tests {
    use prono_api::Answer;

    use super::*;
    use crate::MemoryRepo;

    const SURVEY_ID: u64 = 1;

    fn submission(user: &str) -> Submission {
        let survey = prono_api::survey(SURVEY_ID).unwrap();
        Submission::new(user, survey, &survey.empty_answers())
    }

    #[tokio::test]
    async fn submitted_answers_show_up_in_results() {
        let repo = MemoryRepo::default();
        submit(&repo, SURVEY_ID, "dev-1", &submission("alice")).await.unwrap();

        let results = results(&repo, SURVEY_ID).await.unwrap();
        let survey = prono_api::survey(SURVEY_ID).unwrap();
        assert_eq!(results.len(), survey.questions.len());
        for question in &survey.questions {
            assert_eq!(results[&question.id].len(), 1);
            assert_eq!(results[&question.id][0].user, "alice");
        }
    }

    #[tokio::test]
    async fn poll_answers_including_other_come_back_in_results() {
        const POLL_SURVEY_ID: u64 = 2;
        let survey = prono_api::survey(POLL_SURVEY_ID).unwrap();
        let poll = survey.questions.first().unwrap();
        assert_eq!(poll.kind, prono_api::AnswerKind::Poll);
        let option = poll.possible_answers[0].clone();

        let repo = MemoryRepo::default();
        for (user, device, vote) in [
            ("Alice Smith", "dev-1", option.as_str()),
            ("Bob Jones", "dev-2", "Haskell"),
        ] {
            let (first, last) = user.split_once(' ').unwrap();
            repo.allow_user(POLL_SURVEY_ID, first, last).await.unwrap();
            let mut answers = survey.empty_answers();
            answers[0] = Answer::Poll(vote.into());
            answers[1] = Answer::Poll(survey.questions[1].possible_answers[0].clone());
            let submission = Submission::new(user, survey, &answers);
            submit(&repo, POLL_SURVEY_ID, device, &submission).await.unwrap();
        }

        let results = results(&repo, POLL_SURVEY_ID).await.unwrap();
        let votes = &results[&poll.id];
        assert_eq!(votes.len(), 2);
        assert!(
            votes
                .iter()
                .all(|vote| vote.answer.kind() == prono_api::AnswerKind::Poll)
        );
        let tally = poll.poll_tally(votes);
        assert_eq!(tally[0], (option, 1));
        assert_eq!(tally.last().unwrap(), &("Haskell".to_string(), 1));

        repo.allow_user(POLL_SURVEY_ID, "Carol", "White").await.unwrap();
        let blank = Submission::new("Carol White", survey, &survey.empty_answers());
        let err = submit(&repo, POLL_SURVEY_ID, "dev-3", &blank).await.unwrap_err();
        assert!(matches!(err, Error::Invalid(Invalid::Answer(_))));
    }

    #[tokio::test]
    async fn answering_twice_fails() {
        let repo = MemoryRepo::default();
        submit(&repo, SURVEY_ID, "dev-1", &submission("alice")).await.unwrap();
        let err = submit(&repo, SURVEY_ID, "dev-1", &submission("alice"))
            .await
            .unwrap_err();
        assert_eq!(err, Error::AnswerExists);
    }

    #[tokio::test]
    async fn name_is_bound_to_its_device() {
        let repo = MemoryRepo::default();
        submit(&repo, SURVEY_ID, "dev-1", &submission("alice")).await.unwrap();
        let err = submit(&repo, SURVEY_ID, "dev-2", &submission("alice"))
            .await
            .unwrap_err();
        assert_eq!(err, Error::DeviceMismatch);
    }

    #[tokio::test]
    async fn device_is_bound_to_one_participant() {
        let repo = MemoryRepo::default();
        submit(&repo, SURVEY_ID, "dev-1", &submission("alice")).await.unwrap();
        let err = submit(&repo, SURVEY_ID, "dev-1", &submission("bob")).await.unwrap_err();
        assert_eq!(err, Error::DeviceInUse);
    }

    #[tokio::test]
    async fn invalid_submissions_are_rejected_before_storing() {
        let repo = MemoryRepo::default();

        let err = submit(&repo, 999, "dev-1", &submission("alice")).await.unwrap_err();
        assert_eq!(err, Error::Invalid(Invalid::UnknownSurvey(999)));

        let err = submit(&repo, SURVEY_ID, "dev-1", &submission("")).await.unwrap_err();
        assert_eq!(err, Error::Invalid(Invalid::UserName));

        let err = submit(&repo, SURVEY_ID, " ", &submission("alice")).await.unwrap_err();
        assert_eq!(err, Error::InvalidDeviceId);

        let mut wrong_kind = submission("alice");
        wrong_kind.answers[0].1 = Answer::Text("soon".into());
        let err = submit(&repo, SURVEY_ID, "dev-1", &wrong_kind).await.unwrap_err();
        assert!(matches!(err, Error::Invalid(Invalid::Answer(_))));

        assert_eq!(repo.users().await.unwrap(), Vec::<String>::new());
    }

    #[tokio::test]
    async fn user_status_tells_known_names_from_new_ones() {
        let repo = MemoryRepo::default();
        assert!(!user_status(&repo, "dev-1", "alice").await.unwrap().answered);

        submit(&repo, SURVEY_ID, "dev-1", &submission("alice")).await.unwrap();
        assert!(user_status(&repo, "dev-1", "alice").await.unwrap().answered);
        // Also from another device: the name is taken.
        assert!(user_status(&repo, "dev-2", "alice").await.unwrap().answered);
        assert!(!user_status(&repo, "dev-2", "bob").await.unwrap().answered);
    }

    #[tokio::test]
    async fn user_status_rejects_bad_input() {
        let repo = MemoryRepo::default();
        let err = user_status(&repo, "dev-1", " alice").await.unwrap_err();
        assert_eq!(err, Error::Invalid(Invalid::UserName));
        let err = user_status(&repo, "", "alice").await.unwrap_err();
        assert_eq!(err, Error::InvalidDeviceId);
    }

    #[tokio::test]
    async fn results_of_unknown_survey_fail() {
        let err = results(&MemoryRepo::default(), 999).await.unwrap_err();
        assert_eq!(err, Error::Invalid(Invalid::UnknownSurvey(999)));
    }

    const LIST_SURVEY_ID: u64 = 2;

    fn list_submission(user: &str) -> Submission {
        let survey = prono_api::survey(LIST_SURVEY_ID).unwrap();
        let answers: Vec<_> = survey
            .questions
            .iter()
            .map(|question| Answer::Poll(question.possible_answers[0].clone()))
            .collect();
        Submission::new(user, survey, &answers)
    }

    #[tokio::test]
    async fn name_list_survey_only_takes_listed_names_ignoring_case() {
        let repo = MemoryRepo::default();
        let err = submit(&repo, LIST_SURVEY_ID, "dev-1", &list_submission("Ada Lovelace"))
            .await
            .unwrap_err();
        assert_eq!(err, Error::NotListed);

        // Listed for another survey does not count.
        repo.allow_user(LIST_SURVEY_ID + 1, "Ada", "Lovelace").await.unwrap();
        assert_eq!(
            submit(&repo, LIST_SURVEY_ID, "dev-1", &list_submission("Ada Lovelace"))
                .await
                .unwrap_err(),
            Error::NotListed
        );

        repo.allow_user(LIST_SURVEY_ID, "Ada", "Lovelace").await.unwrap();
        submit(&repo, LIST_SURVEY_ID, "dev-1", &list_submission("ada LOVELACE"))
            .await
            .unwrap();
        // Nothing was stored for the refused attempts.
        assert_eq!(repo.users().await.unwrap(), ["ada LOVELACE"]);
    }

    #[tokio::test]
    async fn user_status_offers_open_surveys_and_the_listed_ones() {
        let repo = MemoryRepo::default();
        let status = user_status(&repo, "dev-1", "Ada Lovelace").await.unwrap();
        assert_eq!(
            status,
            UserStatus {
                answered: false,
                surveys: vec![SURVEY_ID]
            }
        );

        repo.allow_user(LIST_SURVEY_ID, "Ada", "Lovelace").await.unwrap();
        let status = user_status(&repo, "dev-1", "Ada Lovelace").await.unwrap();
        assert_eq!(status.surveys, [SURVEY_ID, LIST_SURVEY_ID]);

        submit(&repo, SURVEY_ID, "dev-1", &submission("Ada Lovelace"))
            .await
            .unwrap();
        assert!(user_status(&repo, "dev-1", "Ada Lovelace").await.unwrap().answered);
    }

    #[tokio::test]
    async fn pass_phrase_survey_needs_the_phrase_and_stays_closed_without_one() {
        // No catalog survey uses a pass phrase yet: check the rule directly.
        let mut survey = prono_api::survey(SURVEY_ID).unwrap().clone();
        survey.access = Access::PassPhrase;
        let plain = submission("Ada Lovelace");
        let repo = MemoryRepo::default();

        // Closed until a pass phrase is set, even for a submission without one.
        assert_eq!(check_access(&repo, &survey, &plain).await, Err(Error::WrongPassPhrase));

        repo.set_pass_phrase(survey.id, "open sesame").await.unwrap();
        assert_eq!(check_access(&repo, &survey, &plain).await, Err(Error::WrongPassPhrase));
        let wrong = plain.clone().with_pass_phrase("nope");
        assert_eq!(check_access(&repo, &survey, &wrong).await, Err(Error::WrongPassPhrase));
        let right = plain.with_pass_phrase(" open sesame ");
        assert_eq!(check_access(&repo, &survey, &right).await, Ok(()));

        // Setting it again replaces it.
        repo.set_pass_phrase(survey.id, "new one").await.unwrap();
        assert_eq!(check_access(&repo, &survey, &right).await, Err(Error::WrongPassPhrase));
    }
}
