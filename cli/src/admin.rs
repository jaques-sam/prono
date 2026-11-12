//! Carries out a [`Commands`] against the repository. The command line and
//! the interactive menu both end up here, so they cannot disagree.

use std::iter::once;

use prono::Repo;
use prono_api::{Answer, Question, Results, Survey};

use crate::commands::{Commands, SurveyAction, UserAction};

/// Runs `command` and returns what to tell the administrator.
///
/// # Errors
///
/// A message for the administrator when the command cannot be carried out.
pub async fn run(repo: &dyn Repo, command: Commands) -> Result<String, String> {
    match command {
        Commands::Surveys { action } => surveys(repo, action).await,
        Commands::Users { action } => users(repo, action).await,
    }
}

async fn surveys(repo: &dyn Repo, action: SurveyAction) -> Result<String, String> {
    match action {
        SurveyAction::List => {
            let surveys = prono::surveys(repo).await.map_err(|e| e.to_string())?;
            if surveys.is_empty() {
                return Ok("No surveys yet. Upload one with `surveys upload <file>`.".into());
            }
            let closed = closed_surveys(repo, &surveys).await?;
            Ok(lines(surveys.iter().map(|survey| {
                let note = if closed.contains(&survey.id) { CLOSED } else { "" };
                format!("{}{note}", describe(survey))
            })))
        }
        SurveyAction::Upload { file } => {
            let json = std::fs::read_to_string(&file).map_err(|e| format!("cannot read {}: {e}", file.display()))?;
            let survey = Survey::from_json(&json).map_err(|e| format!("invalid survey in {}: {e}", file.display()))?;
            let replaced = repo
                .survey_definition(survey.id)
                .await
                .map_err(|e| e.to_string())?
                .is_some();
            prono::upload_survey(repo, &json).await.map_err(|e| e.to_string())?;
            let verb = if replaced { "replaced" } else { "uploaded" };
            let message = format!("Survey {verb}: {}", describe(&survey));
            if closed_surveys(repo, std::slice::from_ref(&survey)).await?.is_empty() {
                return Ok(message);
            }
            Ok(format!(
                "{message}\nNobody can open it until it has a pass phrase: `surveys pass-phrase {} <pass phrase>`.",
                survey.id
            ))
        }
        SurveyAction::PassPhrase { survey_id, pass_phrase } => set_pass_phrase(repo, survey_id, &pass_phrase).await,
        SurveyAction::Scores { survey_id } => {
            let scores = prono::scores(repo, survey_id).await.map_err(|e| e.to_string())?;
            if scores.is_empty() {
                return Ok(format!("Nobody answered survey {survey_id} yet."));
            }
            Ok(lines(
                scores.iter().map(|score| format!("{}: {}", score.user, score.points)),
            ))
        }
        SurveyAction::Answers { survey_id, user } => {
            let survey = prono::survey(repo, survey_id).await.map_err(|e| e.to_string())?;
            let results = prono::results(repo, survey_id).await.map_err(|e| e.to_string())?;
            answers(&survey, &results, user.as_deref())
        }
    }
}

/// Appended to a survey that nobody can open yet.
const CLOSED: &str = " [closed: no pass phrase]";

/// The ids of those of `surveys` that have no pass phrase.
async fn closed_surveys(repo: &dyn Repo, surveys: &[Survey]) -> Result<Vec<u64>, String> {
    let pass_phrases = repo.pass_phrases().await.map_err(|e| e.to_string())?;
    Ok(surveys
        .iter()
        .map(|survey| survey.id)
        .filter(|id| !pass_phrases.iter().any(|(with, _)| with == id))
        .collect())
}

/// Sets the pass phrase of survey `survey_id` and tells which surveys it
/// opens now: those that share it are shown together.
async fn set_pass_phrase(repo: &dyn Repo, survey_id: u64, pass_phrase: &str) -> Result<String, String> {
    prono::survey(repo, survey_id).await.map_err(|e| e.to_string())?;
    let pass_phrase = pass_phrase.trim();
    if pass_phrase.is_empty() {
        return Err("the pass phrase must not be blank".into());
    }
    repo.set_pass_phrase(survey_id, pass_phrase)
        .await
        .map_err(|e| e.to_string())?;
    let mut opened: Vec<_> = repo
        .pass_phrases()
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|(_, stored)| stored == pass_phrase)
        .map(|(id, _)| id)
        .collect();
    opened.sort_unstable();
    let opened = opened.iter().map(u64::to_string).collect::<Vec<_>>().join(", ");
    Ok(format!(
        "Pass phrase of survey {survey_id} set. It opens survey(s) {opened}."
    ))
}

/// The names of everybody who answered survey `survey_id`, in order.
///
/// # Errors
///
/// A message for the administrator when the answers cannot be read.
pub async fn participants(repo: &dyn Repo, survey_id: u64) -> Result<Vec<String>, String> {
    let results = prono::results(repo, survey_id).await.map_err(|e| e.to_string())?;
    Ok(participants_in(&results).into_iter().map(str::to_string).collect())
}

fn participants_in(results: &Results) -> Vec<&str> {
    let mut users: Vec<_> = results.values().flatten().map(|found| found.user.as_str()).collect();
    users.sort_unstable();
    users.dedup();
    users
}

/// The answers of `user` (matched ignoring case), or a table of everybody's.
fn answers(survey: &Survey, results: &Results, user: Option<&str>) -> Result<String, String> {
    let users = participants_in(results);
    let Some(user) = user.map(str::trim) else {
        if users.is_empty() {
            return Ok(format!("Nobody answered survey {} yet.", survey.id));
        }
        return Ok(table(survey, results, &users));
    };
    let user = users
        .into_iter()
        .find(|name| name.to_lowercase() == user.to_lowercase())
        .ok_or_else(|| format!("'{user}' did not answer survey {}", survey.id))?;
    Ok(lines(survey.questions.iter().map(|question| {
        format!("{}: {}", question.text, cell(answer_of(results, question, user)))
    })))
}

/// The questions numbered, then a row per participant and a column per
/// question (`-` where they did not answer).
fn table(survey: &Survey, results: &Results, users: &[&str]) -> String {
    let legend = survey
        .questions
        .iter()
        .enumerate()
        .map(|(index, question)| format!("Q{}: {}", index + 1, question.text));
    let header: Vec<String> = once("Participant".to_string())
        .chain((1..=survey.questions.len()).map(|number| format!("Q{number}")))
        .collect();
    let rows: Vec<Vec<String>> = users
        .iter()
        .map(|user| {
            once((*user).to_string())
                .chain(
                    survey
                        .questions
                        .iter()
                        .map(|question| cell(answer_of(results, question, user))),
                )
                .collect()
        })
        .collect();
    let widths: Vec<usize> = (0..header.len())
        .map(|column| {
            once(&header)
                .chain(&rows)
                .map(|row| row[column].chars().count())
                .max()
                .unwrap_or_default()
        })
        .collect();
    let format_row = |row: &[String]| {
        let padded = row.iter().zip(&widths).map(|(text, width)| format!("{text:<width$}"));
        padded.collect::<Vec<_>>().join(" | ").trim_end().to_string()
    };
    let separator = widths
        .iter()
        .map(|width| "-".repeat(*width))
        .collect::<Vec<_>>()
        .join("-+-");
    let body = once(format_row(&header))
        .chain(once(separator))
        .chain(rows.iter().map(|row| format_row(row)));
    format!("{}\n\n{}", lines(legend), lines(body))
}

fn answer_of<'a>(results: &'a Results, question: &Question, user: &str) -> Option<&'a Answer> {
    let found = results.get(&question.id)?.iter().find(|found| found.user == user)?;
    Some(&found.answer)
}

/// An answer on one line, `-` for none.
fn cell(answer: Option<&Answer>) -> String {
    answer.map_or_else(
        || "-".to_string(),
        |answer| answer.to_string().split_whitespace().collect::<Vec<_>>().join(" "),
    )
}

/// `1: SpaceX survey (4 questions, scored)`
#[must_use]
pub fn describe(survey: &Survey) -> String {
    let scored = if survey.scored { ", scored" } else { "" };
    format!(
        "{}: {} ({} questions{scored})",
        survey.id,
        survey.title,
        survey.questions.len()
    )
}

async fn users(repo: &dyn Repo, action: UserAction) -> Result<String, String> {
    match action {
        UserAction::Show => {
            let users = repo.users().await.map_err(|e| e.to_string())?;
            if users.is_empty() {
                return Ok("No users found.".into());
            }
            Ok(lines(users))
        }
        UserAction::Delete { name } => {
            repo.delete_user(&name).await.map_err(|e| e.to_string())?;
            Ok(format!("User '{name}' deleted."))
        }
    }
}

fn lines(items: impl IntoIterator<Item = String>) -> String {
    items.into_iter().collect::<Vec<_>>().join("\n")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use prono::MemoryRepo;
    use prono_api::samples;

    use super::*;

    fn repo() -> MemoryRepo {
        MemoryRepo::with_surveys([samples::STARSHIP_JSON, samples::COURSE_JSON])
    }

    async fn ok(repo: &MemoryRepo, command: Commands) -> String {
        run(repo, command).await.unwrap()
    }

    async fn err(repo: &MemoryRepo, command: Commands) -> String {
        run(repo, command).await.unwrap_err()
    }

    fn survey_action(action: SurveyAction) -> Commands {
        Commands::Surveys { action }
    }

    fn pass_phrase(survey_id: u64, pass_phrase: &str) -> Commands {
        survey_action(SurveyAction::PassPhrase {
            survey_id,
            pass_phrase: pass_phrase.into(),
        })
    }

    /// A survey file of its own in the temp directory.
    fn survey_file(name: &str, content: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("prono-cli-{}-{name}", std::process::id()));
        std::fs::write(&path, content).unwrap();
        path
    }

    #[tokio::test]
    async fn surveys_are_listed_and_those_without_a_pass_phrase_say_so() {
        let empty = MemoryRepo::default();
        let message = ok(&empty, survey_action(SurveyAction::List)).await;
        assert!(message.contains("No surveys yet"), "{message}");

        let repo = repo();
        let listed = ok(&repo, survey_action(SurveyAction::List)).await;
        let expected = [
            "1: SpaceX Starship (4 questions) [closed: no pass phrase]",
            "2: Rust course (4 questions) [closed: no pass phrase]",
        ];
        assert_eq!(listed, expected.join("\n"));

        ok(&repo, pass_phrase(1, "open sesame")).await;
        let listed = ok(&repo, survey_action(SurveyAction::List)).await;
        let expected = [
            "1: SpaceX Starship (4 questions)",
            "2: Rust course (4 questions) [closed: no pass phrase]",
        ];
        assert_eq!(listed, expected.join("\n"));
    }

    #[tokio::test]
    async fn uploading_a_survey_file_stores_or_replaces_it() {
        let repo = MemoryRepo::default();
        let file = survey_file("upload.json", samples::STARSHIP_JSON);
        let message = ok(&repo, survey_action(SurveyAction::Upload { file: file.clone() })).await;
        assert!(message.starts_with("Survey uploaded: 1:"), "{message}");
        assert!(message.contains("`surveys pass-phrase 1 <pass phrase>`"), "{message}");
        assert_eq!(prono::survey(&repo, 1).await.unwrap(), samples::starship());

        // Replacing keeps the pass phrase, so there is nothing to remind of.
        ok(&repo, pass_phrase(1, "open sesame")).await;
        let message = ok(&repo, survey_action(SurveyAction::Upload { file: file.clone() })).await;
        assert!(message.starts_with("Survey replaced: 1:"), "{message}");
        assert!(!message.contains("pass-phrase"), "{message}");
        std::fs::remove_file(file).unwrap();
    }

    #[tokio::test]
    async fn invalid_or_missing_survey_files_are_reported() {
        let repo = MemoryRepo::default();
        let with_access = samples::STARSHIP_JSON.replace(r#""link""#, r#""access": "pass_phrase", "link""#);
        let file = survey_file("with-access.json", &with_access);
        let message = err(&repo, survey_action(SurveyAction::Upload { file: file.clone() })).await;
        assert!(
            message.contains("invalid survey") && message.contains("access"),
            "{message}"
        );
        std::fs::remove_file(file).unwrap();

        let missing = PathBuf::from("/nonexistent/survey.json");
        let message = err(&repo, survey_action(SurveyAction::Upload { file: missing })).await;
        assert!(message.contains("cannot read"), "{message}");
        assert_eq!(prono::surveys(&repo).await.unwrap(), Vec::<Survey>::new());
    }

    #[tokio::test]
    async fn scores_need_a_scored_survey_and_answers() {
        let quiz = r#"{"survey_id": 3, "title": "Quiz", "description": "A quiz",
            "questions": [{"question": "Q?", "answer_type": "text", "correct_answer": "a"}]}"#;
        let repo = MemoryRepo::with_surveys([samples::STARSHIP_JSON, quiz]);
        repo.set_pass_phrase(3, "open sesame").await.unwrap();
        let scores = |survey_id| survey_action(SurveyAction::Scores { survey_id });

        assert!(err(&repo, scores(1)).await.contains("no scores"));
        assert!(err(&repo, scores(9)).await.contains("Unknown survey"));
        assert_eq!(ok(&repo, scores(3)).await, "Nobody answered survey 3 yet.");

        let survey = prono::survey(&repo, 3).await.unwrap();
        let submission = prono_api::Submission::new("Ann Lee", &survey, &[prono_api::Answer::Text("A".into())])
            .with_pass_phrase("open sesame");
        prono::submit(&repo, 3, "dev-1", &submission).await.unwrap();
        assert_eq!(ok(&repo, scores(3)).await, "Ann Lee: 1");
    }

    /// `name`'s answers to the course survey: the given ones to the first two (poll)
    /// questions, the sample ones to the rest.
    async fn answered_course(repo: &MemoryRepo, name: &str, device: &str, answers_to_first_two: [&str; 2]) {
        repo.set_pass_phrase(2, "rust rocks").await.unwrap();
        let survey = prono::survey(repo, 2).await.unwrap();
        let mut answers = samples::course_answers();
        for (slot, answer) in answers.iter_mut().zip(answers_to_first_two) {
            *slot = Answer::Poll(answer.into());
        }
        let submission = prono_api::Submission::new(name, &survey, &answers).with_pass_phrase("rust rocks");
        prono::submit(repo, 2, device, &submission).await.unwrap();
    }

    fn answers_of(survey_id: u64, user: Option<&str>) -> Commands {
        survey_action(SurveyAction::Answers {
            survey_id,
            user: user.map(Into::into),
        })
    }

    #[tokio::test]
    async fn answers_of_all_participants_are_a_table() {
        let repo = repo();
        assert_eq!(ok(&repo, answers_of(2, None)).await, "Nobody answered survey 2 yet.");

        answered_course(&repo, "Bob Ray", "dev-2", ["C++", "I can't make it 😪"]).await;
        answered_course(&repo, "Ann Lee", "dev-1", ["C", "Of course!"]).await;
        let expected = [
            "Q1: What is your background",
            "Q2: I come to the course!",
            "Q3: Do you have any diatery requirements or allergies?",
            "Q4: Other requests or questions?",
            "",
            "Participant | Q1  | Q2                | Q3 | Q4",
            "------------+-----+-------------------+----+-----------",
            "Ann Lee     | C   | Of course!        | no | No remarks",
            "Bob Ray     | C++ | I can't make it 😪 | no | No remarks",
        ];
        assert_eq!(ok(&repo, answers_of(2, None)).await, expected.join("\n"));
    }

    #[tokio::test]
    async fn a_missing_answer_is_a_dash_and_a_line_break_a_space() {
        let quiz = r#"{"survey_id": 3, "title": "Quiz", "description": "A quiz",
            "questions": [{"question": "A?", "answer_type": "text"}, {"question": "B?", "answer_type": "text"}]}"#;
        let repo = MemoryRepo::with_surveys([quiz]);
        let survey = prono::survey(&repo, 3).await.unwrap();
        let (a, b) = (survey.questions[0].id, survey.questions[1].id);
        repo.save_answers("ann", "dev-1", &[(a, Answer::Text("one\ntwo".into()))])
            .await
            .unwrap();
        repo.save_answers("bob", "dev-2", &[(b, Answer::Text("three".into()))])
            .await
            .unwrap();

        let expected = [
            "Q1: A?",
            "Q2: B?",
            "",
            "Participant | Q1      | Q2",
            "------------+---------+------",
            "ann         | one two | -",
            "bob         | -       | three",
        ];
        assert_eq!(ok(&repo, answers_of(3, None)).await, expected.join("\n"));
        assert_eq!(ok(&repo, answers_of(3, Some("bob"))).await, "A?: -\nB?: three");
    }

    #[tokio::test]
    async fn the_answers_of_one_participant_are_listed_by_question() {
        let repo = repo();
        answered_course(&repo, "Ann Lee", "dev-1", ["C", "Of course!"]).await;
        answered_course(&repo, "Bob Ray", "dev-2", ["C++", "Of course!"]).await;

        let expected = "What is your background: C\nI come to the course!: Of course!\nDo you have any diatery requirements or allergies?: no\nOther requests or questions?: No remarks";
        assert_eq!(ok(&repo, answers_of(2, Some("Ann Lee"))).await, expected);
        // Names match the way they do when answering: ignoring case and surrounding spaces.
        assert_eq!(ok(&repo, answers_of(2, Some(" ann lee "))).await, expected);

        assert_eq!(
            err(&repo, answers_of(2, Some("Cy Young"))).await,
            "'Cy Young' did not answer survey 2"
        );
        assert!(err(&repo, answers_of(9, None)).await.contains("Unknown survey 9"));
    }

    #[tokio::test]
    async fn users_are_shown_and_deleted() {
        let repo = repo();
        let show = || Commands::Users {
            action: UserAction::Show,
        };
        assert_eq!(ok(&repo, show()).await, "No users found.");

        repo.save_answers("alice", "dev-1", &[]).await.unwrap();
        repo.save_answers("bob", "dev-2", &[]).await.unwrap();
        assert_eq!(ok(&repo, show()).await, "alice\nbob");

        let delete = Commands::Users {
            action: UserAction::Delete { name: "alice".into() },
        };
        assert_eq!(ok(&repo, delete).await, "User 'alice' deleted.");
        assert_eq!(ok(&repo, show()).await, "bob");
    }

    #[tokio::test]
    async fn the_pass_phrase_is_set_trimmed_and_never_blank() {
        let repo = repo();
        assert_eq!(
            ok(&repo, pass_phrase(1, " open sesame ")).await,
            "Pass phrase of survey 1 set. It opens survey(s) 1."
        );
        assert_eq!(repo.pass_phrases().await.unwrap(), [(1, "open sesame".to_string())]);
        assert!(err(&repo, pass_phrase(1, "  ")).await.contains("blank"));
        assert_eq!(repo.pass_phrases().await.unwrap(), [(1, "open sesame".to_string())]);

        // Setting it again replaces it.
        ok(&repo, pass_phrase(1, "new one")).await;
        assert_eq!(repo.pass_phrases().await.unwrap(), [(1, "new one".to_string())]);
    }

    #[tokio::test]
    async fn setting_a_pass_phrase_tells_which_surveys_share_it() {
        let repo = repo();
        ok(&repo, pass_phrase(1, "open sesame")).await;
        assert_eq!(
            ok(&repo, pass_phrase(2, "open sesame")).await,
            "Pass phrase of survey 2 set. It opens survey(s) 1, 2."
        );
    }

    #[tokio::test]
    async fn only_a_survey_that_exists_gets_a_pass_phrase() {
        let repo = repo();
        assert!(err(&repo, pass_phrase(999, "x")).await.contains("Unknown survey 999"));
        assert_eq!(repo.pass_phrases().await.unwrap(), Vec::<(u64, String)>::new());
    }
}
