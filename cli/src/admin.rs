//! Carries out a [`Commands`] against the repository. The command line and
//! the interactive menu both end up here, so they cannot disagree.

use std::iter::once;

use prono::Repo;
use prono_api::{Access, Answer, Question, Results, Survey, check_name_parts};

use crate::commands::{AccessAction, Commands, SurveyAction, UserAction};

/// Runs `command` and returns what to tell the administrator.
///
/// # Errors
///
/// A message for the administrator when the command cannot be carried out.
pub async fn run(repo: &dyn Repo, command: Commands) -> Result<String, String> {
    match command {
        Commands::Surveys { action } => surveys(repo, action).await,
        Commands::Users { action } => users(repo, action).await,
        Commands::Access { action } => access(repo, action).await,
    }
}

async fn surveys(repo: &dyn Repo, action: SurveyAction) -> Result<String, String> {
    match action {
        SurveyAction::List => {
            let surveys = prono::surveys(repo).await.map_err(|e| e.to_string())?;
            if surveys.is_empty() {
                return Ok("No surveys yet. Upload one with `surveys upload <file>`.".into());
            }
            Ok(lines(surveys.iter().map(describe)))
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
            Ok(format!("Survey {verb}: {}", describe(&survey)))
        }
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

/// `1: SpaceX survey (pass phrase, 4 questions, scored)`
#[must_use]
pub fn describe(survey: &Survey) -> String {
    let access = match survey.access {
        Access::PassPhrase => "pass phrase",
        Access::NameList => "name list",
    };
    let scored = if survey.scored { ", scored" } else { "" };
    format!(
        "{}: {} ({access}, {} questions{scored})",
        survey.id,
        survey.description,
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

async fn access(repo: &dyn Repo, action: AccessAction) -> Result<String, String> {
    match action {
        AccessAction::Allow {
            survey_id,
            first_name,
            last_name,
        } => {
            require_access(repo, survey_id, Access::NameList).await?;
            let name = check_name_parts(&first_name, &last_name).map_err(|e| e.to_string())?;
            repo.allow_user(survey_id, first_name.trim(), last_name.trim())
                .await
                .map_err(|e| e.to_string())?;
            Ok(format!("'{name}' may answer survey {survey_id}."))
        }
        AccessAction::Disallow {
            survey_id,
            first_name,
            last_name,
        } => {
            require_access(repo, survey_id, Access::NameList).await?;
            repo.disallow_user(survey_id, first_name.trim(), last_name.trim())
                .await
                .map_err(|e| e.to_string())?;
            Ok(format!(
                "'{} {}' may no longer answer survey {survey_id}.",
                first_name.trim(),
                last_name.trim()
            ))
        }
        AccessAction::Show { survey_id } => {
            require_access(repo, survey_id, Access::NameList).await?;
            let names = repo.allowed_users(survey_id).await.map_err(|e| e.to_string())?;
            if names.is_empty() {
                return Ok(format!("Nobody is on the list of survey {survey_id}."));
            }
            Ok(lines(names.iter().map(|(first, last)| format!("{first} {last}"))))
        }
        AccessAction::PassPhrase { survey_id, pass_phrase } => {
            require_access(repo, survey_id, Access::PassPhrase).await?;
            let pass_phrase = pass_phrase.trim();
            if pass_phrase.is_empty() {
                return Err("the pass phrase must not be blank".into());
            }
            repo.set_pass_phrase(survey_id, pass_phrase)
                .await
                .map_err(|e| e.to_string())?;
            Ok(format!("Pass phrase of survey {survey_id} set."))
        }
    }
}

/// Fails unless survey `survey_id` exists and uses `access`: a list or pass
/// phrase for a survey that does not check it would silently do nothing.
async fn require_access(repo: &dyn Repo, survey_id: u64, access: Access) -> Result<(), String> {
    let survey = prono::survey(repo, survey_id).await.map_err(|e| e.to_string())?;
    if survey.access == access {
        Ok(())
    } else {
        Err(format!(
            "survey {survey_id} has access {:?}, not {access:?}",
            survey.access
        ))
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

    fn access_action(action: AccessAction) -> Commands {
        Commands::Access { action }
    }

    fn allow(survey_id: u64, first: &str, last: &str) -> AccessAction {
        AccessAction::Allow {
            survey_id,
            first_name: first.into(),
            last_name: last.into(),
        }
    }

    /// A survey file of its own in the temp directory.
    fn survey_file(name: &str, content: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("prono-cli-{}-{name}", std::process::id()));
        std::fs::write(&path, content).unwrap();
        path
    }

    #[tokio::test]
    async fn surveys_are_listed_with_their_access() {
        let empty = MemoryRepo::default();
        let message = ok(&empty, survey_action(SurveyAction::List)).await;
        assert!(message.contains("No surveys yet"), "{message}");

        let listed = ok(&repo(), survey_action(SurveyAction::List)).await;
        let expected = [
            "1: Starship achievements survey (pass phrase, 4 questions)",
            "2: You were chosen to follow a splendid course... (name list, 4 questions)",
        ];
        assert_eq!(listed, expected.join("\n"));
    }

    #[tokio::test]
    async fn uploading_a_survey_file_stores_or_replaces_it() {
        let repo = MemoryRepo::default();
        let file = survey_file("upload.json", samples::STARSHIP_JSON);
        let message = ok(&repo, survey_action(SurveyAction::Upload { file: file.clone() })).await;
        assert!(message.starts_with("Survey uploaded: 1:"), "{message}");
        assert_eq!(prono::survey(&repo, 1).await.unwrap(), samples::starship());

        let message = ok(&repo, survey_action(SurveyAction::Upload { file: file.clone() })).await;
        assert!(message.starts_with("Survey replaced: 1:"), "{message}");
        std::fs::remove_file(file).unwrap();
    }

    #[tokio::test]
    async fn invalid_or_missing_survey_files_are_reported() {
        let repo = MemoryRepo::default();
        let no_access = samples::STARSHIP_JSON.replace(r#""access": "pass_phrase","#, "");
        let file = survey_file("no-access.json", &no_access);
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
        let quiz = r#"{"survey_id": 3, "description": "Quiz", "access": "name_list",
            "questions": [{"question": "Q?", "answer_type": "text", "correct_answer": "a"}]}"#;
        let repo = MemoryRepo::with_surveys([samples::STARSHIP_JSON, quiz]);
        let scores = |survey_id| survey_action(SurveyAction::Scores { survey_id });

        assert!(err(&repo, scores(1)).await.contains("no scores"));
        assert!(err(&repo, scores(9)).await.contains("Unknown survey"));
        assert_eq!(ok(&repo, scores(3)).await, "Nobody answered survey 3 yet.");

        let survey = prono::survey(&repo, 3).await.unwrap();
        repo.allow_user(3, "Ann", "Lee").await.unwrap();
        let submission = prono_api::Submission::new("Ann Lee", &survey, &[prono_api::Answer::Text("A".into())]);
        prono::submit(&repo, 3, "dev-1", &submission).await.unwrap();
        assert_eq!(ok(&repo, scores(3)).await, "Ann Lee: 1");
    }

    /// `name` on the name list of the course survey, and their answers to it: the given
    /// ones to the first two (poll) questions, the sample ones to the rest.
    async fn answered_course(repo: &MemoryRepo, name: &str, device: &str, answers_to_first_two: [&str; 2]) {
        let (first, last) = name.split_once(' ').unwrap();
        repo.allow_user(2, first, last).await.unwrap();
        let survey = prono::survey(repo, 2).await.unwrap();
        let mut answers = samples::course_answers();
        for (slot, answer) in answers.iter_mut().zip(answers_to_first_two) {
            *slot = Answer::Poll(answer.into());
        }
        let submission = prono_api::Submission::new(name, &survey, &answers);
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
        let quiz = r#"{"survey_id": 3, "description": "Quiz", "access": "name_list",
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
    async fn the_name_list_is_filled_and_emptied() {
        let repo = repo();
        let show = || access_action(AccessAction::Show { survey_id: 2 });
        assert_eq!(ok(&repo, show()).await, "Nobody is on the list of survey 2.");

        let message = ok(&repo, access_action(allow(2, " Ada ", "Lovelace"))).await;
        assert_eq!(message, "'Ada Lovelace' may answer survey 2.");
        ok(&repo, access_action(allow(2, "Alan", "Turing"))).await;
        assert_eq!(ok(&repo, show()).await, "Ada Lovelace\nAlan Turing");

        let disallow = access_action(AccessAction::Disallow {
            survey_id: 2,
            first_name: "Ada".into(),
            last_name: "Lovelace".into(),
        });
        assert_eq!(
            ok(&repo, disallow).await,
            "'Ada Lovelace' may no longer answer survey 2."
        );
        assert_eq!(ok(&repo, show()).await, "Alan Turing");
    }

    #[tokio::test]
    async fn a_name_needs_both_parts() {
        let repo = repo();
        let message = err(&repo, access_action(allow(2, "Ada", "  "))).await;
        assert!(message.contains("first and last name"), "{message}");
        assert_eq!(repo.allowed_users(2).await.unwrap(), Vec::<(String, String)>::new());
    }

    #[tokio::test]
    async fn the_pass_phrase_is_set_trimmed_and_never_blank() {
        let repo = repo();
        let set = |phrase: &str| {
            access_action(AccessAction::PassPhrase {
                survey_id: 1,
                pass_phrase: phrase.into(),
            })
        };
        assert_eq!(ok(&repo, set(" open sesame ")).await, "Pass phrase of survey 1 set.");
        assert_eq!(repo.pass_phrase(1).await.unwrap().as_deref(), Some("open sesame"));
        assert!(err(&repo, set("  ")).await.contains("blank"));
        assert_eq!(repo.pass_phrase(1).await.unwrap().as_deref(), Some("open sesame"));
    }

    #[tokio::test]
    async fn access_commands_only_fit_surveys_using_that_access() {
        let repo = repo();
        let show = |survey_id| access_action(AccessAction::Show { survey_id });
        assert!(err(&repo, show(1)).await.contains("PassPhrase, not NameList"));
        assert!(err(&repo, show(999)).await.contains("Unknown survey 999"));
        let phrase = access_action(AccessAction::PassPhrase {
            survey_id: 2,
            pass_phrase: "x".into(),
        });
        assert!(err(&repo, phrase).await.contains("NameList, not PassPhrase"));
        assert!(repo.pass_phrase(2).await.unwrap().is_none());
    }
}
