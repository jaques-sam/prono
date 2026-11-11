use log::info;
use prono_api::{Access, Invalid, Results, Score, Submission, Survey, UserStatus, check_user_name};

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
    let survey = survey(repo, survey_id).await?;
    submission.validate(&survey)?;
    check_device_id(device_id)?;
    check_access(repo, &survey, submission).await?;

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

/// Checks the pass phrase `pass_phrase` (see [`prono_api::clean_pass_phrase`])
/// of survey `survey_id`, so the app can refuse a wrong one before the
/// questions open. A survey without a pass phrase, set by nobody or not
/// asking for one, never accepts any.
///
/// # Errors
///
/// [`Error::WrongPassPhrase`], or a repository error.
pub async fn check_pass_phrase(repo: &dyn Repo, survey_id: u64, pass_phrase: Option<&str>) -> Result<()> {
    if pass_phrase_is_right(repo, survey_id, pass_phrase).await? {
        Ok(())
    } else {
        Err(Error::WrongPassPhrase)
    }
}

/// Whether `pass_phrase` is the one set for survey `survey_id`.
async fn pass_phrase_is_right(repo: &dyn Repo, survey_id: u64, pass_phrase: Option<&str>) -> Result<bool> {
    let stored = repo.pass_phrase(survey_id).await?;
    Ok(stored.is_some() && stored.as_deref() == pass_phrase)
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
        surveys: surveys(repo)
            .await?
            .into_iter()
            .filter(|survey| survey.access != Access::NameList || listed.contains(&survey.id))
            .collect(),
    })
}

/// Every participant's answers to survey `survey_id`.
///
/// # Errors
///
/// [`Error::Invalid`] for an unknown survey, or a repository error.
pub async fn results(repo: &dyn Repo, survey_id: u64) -> Result<Results> {
    collect_results(repo, &survey(repo, survey_id).await?).await
}

/// What each participant got right in survey `survey_id`, best first.
///
/// # Errors
///
/// [`Error::Invalid`] for an unknown survey or one that is not scored, or a
/// repository error.
pub async fn scores(repo: &dyn Repo, survey_id: u64) -> Result<Vec<Score>> {
    let survey = survey(repo, survey_id).await?;
    if !survey.scored {
        return Err(Invalid::NotScored(survey_id).into());
    }
    Ok(survey.scores(&collect_results(repo, &survey).await?))
}

/// Checks the survey definition `json` and stores it, replacing the survey
/// with the same id. Answers already given stay: they follow the questions
/// by id, and an id only changes with the text or the kind of a question.
///
/// # Errors
///
/// [`Error::InvalidSurvey`] if the definition is not valid, or a repository
/// error.
pub async fn upload_survey(repo: &dyn Repo, json: &str) -> Result<Survey> {
    let survey = Survey::from_json(json).map_err(|err| Error::InvalidSurvey(err.to_string()))?;
    repo.save_survey_definition(survey.id, json).await?;
    info!("Uploaded survey {}", survey.id);
    Ok(survey)
}

/// Survey `survey_id`, with its correct answers if it has any.
///
/// # Errors
///
/// [`Error::Invalid`] for an unknown survey, or a repository error.
pub async fn survey(repo: &dyn Repo, survey_id: u64) -> Result<Survey> {
    let definition = repo
        .survey_definition(survey_id)
        .await?
        .ok_or(Error::Invalid(Invalid::UnknownSurvey(survey_id)))?;
    stored(&definition)
}

/// All surveys, with their correct answers if they have any, by id.
///
/// # Errors
///
/// A repository error.
pub async fn surveys(repo: &dyn Repo) -> Result<Vec<Survey>> {
    let mut surveys = repo
        .survey_definitions()
        .await?
        .iter()
        .map(|definition| stored(definition))
        .collect::<Result<Vec<_>>>()?;
    surveys.sort_by_key(|survey| survey.id);
    Ok(surveys)
}

/// A stored definition was checked when it was uploaded, so it failing to
/// parse means the database was changed by hand.
fn stored(definition: &str) -> Result<Survey> {
    Survey::from_json(definition).map_err(|err| Error::Repository(format!("invalid stored survey: {err}")))
}

async fn collect_results(repo: &dyn Repo, survey: &Survey) -> Result<Results> {
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
        Access::NameList => repo.listed_surveys(&submission.user).await?.contains(&survey.id),
        Access::PassPhrase => pass_phrase_is_right(repo, survey.id, submission.pass_phrase.as_deref()).await?,
    };
    match (allowed, survey.access) {
        (true, _) => Ok(()),
        (false, Access::PassPhrase) => Err(Error::WrongPassPhrase),
        (false, Access::NameList) => Err(Error::NotListed),
    }
}

fn check_device_id(device_id: &str) -> Result<()> {
    if device_id.trim().is_empty() || device_id.len() > MAX_DEVICE_ID_LEN {
        return Err(Error::InvalidDeviceId);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use prono_api::{Answer, AnswerKind, samples};

    use super::*;
    use crate::MemoryRepo;

    /// The survey with a pass phrase.
    const SURVEY_ID: u64 = 1;
    /// The survey with a name list.
    const LIST_SURVEY_ID: u64 = 2;
    const PASS_PHRASE: &str = "open sesame";

    /// A repository with both sample surveys, and the pass phrase of the first.
    async fn repo() -> MemoryRepo {
        let repo = MemoryRepo::with_surveys([samples::STARSHIP_JSON, samples::COURSE_JSON]);
        repo.set_pass_phrase(SURVEY_ID, PASS_PHRASE).await.unwrap();
        repo
    }

    /// `user`'s blank submission to the first survey, with the right pass phrase.
    fn submission(user: &str) -> Submission {
        let survey = samples::starship();
        Submission::new(user, &survey, &survey.empty_answers()).with_pass_phrase(PASS_PHRASE)
    }

    fn list_submission(user: &str) -> Submission {
        let survey = samples::course();
        Submission::new(user, &survey, &samples::course_answers())
    }

    fn ids(surveys: &[Survey]) -> Vec<u64> {
        surveys.iter().map(|survey| survey.id).collect()
    }

    #[tokio::test]
    async fn submitted_answers_show_up_in_results() {
        let repo = repo().await;
        submit(&repo, SURVEY_ID, "dev-1", &submission("alice")).await.unwrap();

        let results = results(&repo, SURVEY_ID).await.unwrap();
        let survey = samples::starship();
        assert_eq!(results.len(), survey.questions.len());
        for question in &survey.questions {
            assert_eq!(results[&question.id].len(), 1);
            assert_eq!(results[&question.id][0].user, "alice");
        }
    }

    #[tokio::test]
    async fn poll_answers_including_other_come_back_in_results() {
        let survey = samples::course();
        let poll = survey.questions.first().unwrap();
        assert_eq!(poll.kind, AnswerKind::Poll);
        let option = poll.possible_answers[0].clone();

        let repo = repo().await;
        for (user, device, vote) in [
            ("Alice Smith", "dev-1", option.as_str()),
            ("Bob Jones", "dev-2", "Haskell"),
        ] {
            let (first, last) = user.split_once(' ').unwrap();
            repo.allow_user(LIST_SURVEY_ID, first, last).await.unwrap();
            let mut answers = samples::course_answers();
            answers[0] = Answer::Poll(vote.into());
            let submission = Submission::new(user, &survey, &answers);
            submit(&repo, LIST_SURVEY_ID, device, &submission).await.unwrap();
        }

        let results = results(&repo, LIST_SURVEY_ID).await.unwrap();
        let votes = &results[&poll.id];
        assert_eq!(votes.len(), 2);
        assert!(votes.iter().all(|vote| vote.answer.kind() == AnswerKind::Poll));
        let tally = poll.poll_tally(votes);
        assert_eq!(tally[0], (option, 1));
        assert_eq!(tally.last().unwrap(), &("Haskell".to_string(), 1));

        repo.allow_user(LIST_SURVEY_ID, "Carol", "White").await.unwrap();
        let blank = Submission::new("Carol White", &survey, &survey.empty_answers());
        let err = submit(&repo, LIST_SURVEY_ID, "dev-3", &blank).await.unwrap_err();
        assert!(matches!(err, Error::Invalid(Invalid::Answer(_))));
    }

    #[tokio::test]
    async fn answering_twice_fails() {
        let repo = repo().await;
        submit(&repo, SURVEY_ID, "dev-1", &submission("alice")).await.unwrap();
        let err = submit(&repo, SURVEY_ID, "dev-1", &submission("alice"))
            .await
            .unwrap_err();
        assert_eq!(err, Error::AnswerExists);
    }

    #[tokio::test]
    async fn name_is_bound_to_its_device() {
        let repo = repo().await;
        submit(&repo, SURVEY_ID, "dev-1", &submission("alice")).await.unwrap();
        let err = submit(&repo, SURVEY_ID, "dev-2", &submission("alice"))
            .await
            .unwrap_err();
        assert_eq!(err, Error::DeviceMismatch);
    }

    #[tokio::test]
    async fn device_is_bound_to_one_participant() {
        let repo = repo().await;
        submit(&repo, SURVEY_ID, "dev-1", &submission("alice")).await.unwrap();
        let err = submit(&repo, SURVEY_ID, "dev-1", &submission("bob")).await.unwrap_err();
        assert_eq!(err, Error::DeviceInUse);
    }

    #[tokio::test]
    async fn invalid_submissions_are_rejected_before_storing() {
        let repo = repo().await;

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
        let repo = repo().await;
        assert!(!user_status(&repo, "dev-1", "alice").await.unwrap().answered);

        submit(&repo, SURVEY_ID, "dev-1", &submission("alice")).await.unwrap();
        assert!(user_status(&repo, "dev-1", "alice").await.unwrap().answered);
        // Also from another device: the name is taken.
        assert!(user_status(&repo, "dev-2", "alice").await.unwrap().answered);
        assert!(!user_status(&repo, "dev-2", "bob").await.unwrap().answered);
    }

    #[tokio::test]
    async fn user_status_rejects_bad_input() {
        let repo = repo().await;
        let err = user_status(&repo, "dev-1", " alice").await.unwrap_err();
        assert_eq!(err, Error::Invalid(Invalid::UserName));
        let err = user_status(&repo, "", "alice").await.unwrap_err();
        assert_eq!(err, Error::InvalidDeviceId);
    }

    #[tokio::test]
    async fn results_of_unknown_survey_fail() {
        let err = results(&repo().await, 999).await.unwrap_err();
        assert_eq!(err, Error::Invalid(Invalid::UnknownSurvey(999)));
    }

    #[tokio::test]
    async fn name_list_survey_only_takes_listed_names_ignoring_case() {
        let repo = repo().await;
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
    async fn user_status_offers_the_surveys_the_name_may_answer() {
        let repo = repo().await;
        // Only the pass phrase survey: the other one has a name list.
        let status = user_status(&repo, "dev-1", "Ada Lovelace").await.unwrap();
        assert_eq!(ids(&status.surveys), [SURVEY_ID]);

        repo.allow_user(LIST_SURVEY_ID, "Ada", "Lovelace").await.unwrap();
        let status = user_status(&repo, "dev-1", "Ada Lovelace").await.unwrap();
        assert_eq!(ids(&status.surveys), [SURVEY_ID, LIST_SURVEY_ID]);

        // Answering does not change what is offered.
        submit(&repo, SURVEY_ID, "dev-1", &submission("Ada Lovelace"))
            .await
            .unwrap();
        let status = user_status(&repo, "dev-1", "Ada Lovelace").await.unwrap();
        assert!(status.answered);
        assert_eq!(ids(&status.surveys), [SURVEY_ID, LIST_SURVEY_ID]);
    }

    #[tokio::test]
    async fn pass_phrase_survey_needs_the_phrase_and_stays_closed_without_one() {
        let plain = submission("Ada Lovelace");
        let mut without = plain.clone();
        without.pass_phrase = None;
        let wrong = plain.clone().with_pass_phrase("nope");

        // Closed until a pass phrase is set, even for a submission without one.
        let repo = MemoryRepo::with_surveys([samples::STARSHIP_JSON]);
        for attempt in [&without, &plain] {
            let err = submit(&repo, SURVEY_ID, "dev-1", attempt).await.unwrap_err();
            assert_eq!(err, Error::WrongPassPhrase);
        }

        repo.set_pass_phrase(SURVEY_ID, PASS_PHRASE).await.unwrap();
        for attempt in [&without, &wrong] {
            let err = submit(&repo, SURVEY_ID, "dev-1", attempt).await.unwrap_err();
            assert_eq!(err, Error::WrongPassPhrase);
        }
        assert_eq!(repo.users().await.unwrap(), Vec::<String>::new());
        let right = plain.clone().with_pass_phrase(&format!(" {PASS_PHRASE} "));
        submit(&repo, SURVEY_ID, "dev-1", &right).await.unwrap();

        // Setting it again replaces it.
        repo.set_pass_phrase(SURVEY_ID, "new one").await.unwrap();
        let err = submit(&repo, SURVEY_ID, "dev-2", &plain).await.unwrap_err();
        assert_eq!(err, Error::WrongPassPhrase);
    }

    #[tokio::test]
    async fn only_the_right_pass_phrase_is_accepted_before_answering() {
        let repo = MemoryRepo::with_surveys([samples::STARSHIP_JSON, samples::COURSE_JSON]);

        // Nobody set one yet: nothing is right, not even none.
        for attempt in [None, Some(PASS_PHRASE)] {
            let err = check_pass_phrase(&repo, SURVEY_ID, attempt).await.unwrap_err();
            assert_eq!(err, Error::WrongPassPhrase);
        }

        repo.set_pass_phrase(SURVEY_ID, PASS_PHRASE).await.unwrap();
        for attempt in [None, Some("nope"), Some("Open Sesame")] {
            let err = check_pass_phrase(&repo, SURVEY_ID, attempt).await.unwrap_err();
            assert_eq!(err, Error::WrongPassPhrase);
        }
        check_pass_phrase(&repo, SURVEY_ID, Some(PASS_PHRASE)).await.unwrap();

        // It is the survey's own: a survey without one accepts none.
        let err = check_pass_phrase(&repo, LIST_SURVEY_ID, Some(PASS_PHRASE))
            .await
            .unwrap_err();
        assert_eq!(err, Error::WrongPassPhrase);
    }

    #[tokio::test]
    async fn uploaded_surveys_are_listed_by_id_and_replaced() {
        let repo = MemoryRepo::default();
        assert_eq!(surveys(&repo).await.unwrap(), Vec::<Survey>::new());
        let err = survey(&repo, SURVEY_ID).await.unwrap_err();
        assert_eq!(err, Error::Invalid(Invalid::UnknownSurvey(SURVEY_ID)));

        // Uploaded out of order.
        let course = upload_survey(&repo, samples::COURSE_JSON).await.unwrap();
        let starship = upload_survey(&repo, samples::STARSHIP_JSON).await.unwrap();
        assert_eq!(course, samples::course());
        assert_eq!(surveys(&repo).await.unwrap(), [starship.clone(), course]);
        assert_eq!(survey(&repo, SURVEY_ID).await.unwrap(), starship);

        let renamed = samples::STARSHIP_JSON.replace("Starship achievements survey", "Renamed");
        upload_survey(&repo, &renamed).await.unwrap();
        assert_eq!(survey(&repo, SURVEY_ID).await.unwrap().description, "Renamed");
        assert_eq!(surveys(&repo).await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn an_invalid_survey_is_not_uploaded() {
        let repo = MemoryRepo::default();
        let no_access = samples::STARSHIP_JSON.replace(r#""access": "pass_phrase","#, "");
        for json in ["nonsense", no_access.as_str()] {
            let err = upload_survey(&repo, json).await.unwrap_err();
            assert!(matches!(err, Error::InvalidSurvey(_)), "{json}");
        }
        assert_eq!(surveys(&repo).await.unwrap(), Vec::<Survey>::new());
    }

    #[tokio::test]
    async fn a_corrupt_stored_definition_is_a_repository_error() {
        let repo = MemoryRepo::default();
        repo.save_survey_definition(SURVEY_ID, "{}").await.unwrap();
        assert!(matches!(survey(&repo, SURVEY_ID).await, Err(Error::Repository(_))));
        assert!(matches!(surveys(&repo).await, Err(Error::Repository(_))));
    }

    const QUIZ: &str = r#"{
        "survey_id": 3,
        "description": "Quiz",
        "access": "pass_phrase",
        "questions": [
            {"question": "Capital of Belgium", "answer_type": "text", "correct_answer": "Brussels"},
            {"question": "Best language", "answer_type": "poll", "possible_answers": ["C", "Rust"],
             "correct_answer": "Rust"}
        ]
    }"#;

    #[tokio::test]
    async fn scores_rank_the_participants_of_a_scored_survey() {
        let repo = MemoryRepo::default();
        let quiz = upload_survey(&repo, QUIZ).await.unwrap();
        repo.set_pass_phrase(quiz.id, PASS_PHRASE).await.unwrap();
        for (user, device, capital, language) in [
            ("Ann", "dev-1", "brussels", "Rust"),
            ("Bob", "dev-2", "Paris", "Rust"),
            ("Cat", "dev-3", "Paris", "C"),
        ] {
            let answers = [Answer::Text(capital.into()), Answer::Poll(language.into())];
            let submission = Submission::new(user, &quiz, &answers).with_pass_phrase(PASS_PHRASE);
            submit(&repo, quiz.id, device, &submission).await.unwrap();
        }

        let points = |user: &str, points| Score {
            user: user.into(),
            points,
        };
        assert_eq!(
            scores(&repo, quiz.id).await.unwrap(),
            [points("Ann", 2), points("Bob", 1), points("Cat", 0)]
        );
    }

    #[tokio::test]
    async fn scores_need_a_scored_survey() {
        let repo = repo().await;
        let err = scores(&repo, SURVEY_ID).await.unwrap_err();
        assert_eq!(err, Error::Invalid(Invalid::NotScored(SURVEY_ID)));
        let err = scores(&repo, 999).await.unwrap_err();
        assert_eq!(err, Error::Invalid(Invalid::UnknownSurvey(999)));
    }

    #[tokio::test]
    async fn adding_the_correct_answers_later_keeps_the_given_answers() {
        let repo = MemoryRepo::default();
        let plain = QUIZ.replace(r#", "correct_answer": "Brussels""#, "").replace(
            r#",
             "correct_answer": "Rust""#,
            "",
        );
        let open = upload_survey(&repo, &plain).await.unwrap();
        assert!(!open.scored);
        repo.set_pass_phrase(open.id, PASS_PHRASE).await.unwrap();
        let answers = [Answer::Text("Brussels".into()), Answer::Poll("Rust".into())];
        let submission = Submission::new("Ann", &open, &answers).with_pass_phrase(PASS_PHRASE);
        submit(&repo, open.id, "dev-1", &submission).await.unwrap();

        let quiz = upload_survey(&repo, QUIZ).await.unwrap();
        assert!(quiz.scored);
        let ann = Score {
            user: "Ann".into(),
            points: 2,
        };
        assert_eq!(scores(&repo, quiz.id).await.unwrap(), [ann]);
    }
}
