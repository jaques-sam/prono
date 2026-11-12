//! The interactive mode: menus to navigate instead of commands to remember.
//! Every menu builds a [`Commands`] and hands it to [`admin::run`], so it does
//! exactly what the command line does. The questions go through a [`Prompt`],
//! which the terminal implements and the tests script.

use std::iter::once;
use std::path::PathBuf;

use prono::Repo;
use prono_api::Survey;

use crate::admin::{self, describe};
use crate::commands::{Commands, SurveyAction, UserAction};

/// How the menus talk to the administrator. Going back (Esc) is `None`.
pub trait Prompt {
    /// Asks to pick one of `options`; returns its index.
    fn select(&mut self, message: &str, options: &[String]) -> Option<usize>;
    /// Asks for a line of text.
    fn text(&mut self, message: &str) -> Option<String>;
    /// Asks to confirm; `false` unless the administrator agrees.
    fn confirm(&mut self, message: &str) -> bool;
    /// Tells something.
    fn show(&mut self, text: &str);
}

/// Shows the main menu until the administrator quits (Esc).
pub async fn run(repo: &dyn Repo, prompt: &mut impl Prompt) {
    while let Some(choice) = pick(prompt, "Prono admin (Esc quits)", &["Surveys", "Users"]) {
        match choice {
            0 => surveys_menu(repo, prompt).await,
            _ => users_menu(repo, prompt).await,
        }
    }
}

fn pick(prompt: &mut impl Prompt, message: &str, options: &[&str]) -> Option<usize> {
    let options: Vec<String> = options.iter().map(|option| (*option).to_string()).collect();
    prompt.select(message, &options)
}

/// Carries out `command` and shows what came of it.
async fn execute(repo: &dyn Repo, prompt: &mut impl Prompt, command: Commands) {
    match admin::run(repo, command).await {
        Ok(message) => prompt.show(&message),
        Err(error) => prompt.show(&format!("error: {error}")),
    }
}

async fn surveys_menu(repo: &dyn Repo, prompt: &mut impl Prompt) {
    let options = [
        "List the surveys",
        "Upload a survey file",
        "Set the pass phrase of a survey",
        "Show the scores of a survey",
        "Show the answers of a survey",
    ];
    while let Some(choice) = pick(prompt, "Surveys (Esc goes back)", &options) {
        let action = match choice {
            0 => SurveyAction::List,
            1 => {
                let Some(file) = prompt.text("Path of the survey JSON file:") else {
                    continue;
                };
                SurveyAction::Upload {
                    file: PathBuf::from(file.trim()),
                }
            }
            2 => {
                let Some(survey) = pick_survey(repo, prompt, "Pass phrase of which survey?", |_| true).await else {
                    continue;
                };
                let Some(pass_phrase) = prompt.text("The new pass phrase:") else {
                    continue;
                };
                SurveyAction::PassPhrase {
                    survey_id: survey.id,
                    pass_phrase,
                }
            }
            3 => {
                let Some(survey) = pick_survey(repo, prompt, "Scores of which survey?", |survey| survey.scored).await
                else {
                    continue;
                };
                SurveyAction::Scores { survey_id: survey.id }
            }
            _ => {
                let Some(survey) = pick_survey(repo, prompt, "Answers of which survey?", |_| true).await else {
                    continue;
                };
                let Some(user) = pick_participant(repo, prompt, survey.id).await else {
                    continue;
                };
                SurveyAction::Answers {
                    survey_id: survey.id,
                    user,
                }
            }
        };
        execute(repo, prompt, Commands::Surveys { action }).await;
    }
}

async fn users_menu(repo: &dyn Repo, prompt: &mut impl Prompt) {
    while let Some(choice) = pick(prompt, "Users (Esc goes back)", &["Show all users", "Delete a user"]) {
        let action = if choice == 0 {
            UserAction::Show
        } else {
            let users = match repo.users().await {
                Ok(users) => users,
                Err(error) => {
                    prompt.show(&format!("error: {error}"));
                    continue;
                }
            };
            let Some(name) = pick_one(prompt, "Delete which user?", users, "No users found.") else {
                continue;
            };
            if !prompt.confirm(&format!("Delete '{name}' and all their answers?")) {
                continue;
            }
            UserAction::Delete { name }
        };
        execute(repo, prompt, Commands::Users { action }).await;
    }
}

/// Asks whose answers to show: `Some(None)` is everybody's, `None` is going
/// back (or nobody answered).
async fn pick_participant(repo: &dyn Repo, prompt: &mut impl Prompt, survey_id: u64) -> Option<Option<String>> {
    let users = match admin::participants(repo, survey_id).await {
        Ok(users) => users,
        Err(error) => {
            prompt.show(&format!("error: {error}"));
            return None;
        }
    };
    if users.is_empty() {
        prompt.show("Nobody answered this survey yet.");
        return None;
    }
    let labels: Vec<_> = once("Everybody (a table)".to_string())
        .chain(users.iter().cloned())
        .collect();
    let index = prompt.select("Whose answers?", &labels)?;
    Some(index.checked_sub(1).map(|user| users[user].clone()))
}

/// Asks to pick one of the surveys that pass `filter`.
async fn pick_survey(
    repo: &dyn Repo,
    prompt: &mut impl Prompt,
    message: &str,
    filter: impl Fn(&Survey) -> bool,
) -> Option<Survey> {
    let surveys = match prono::surveys(repo).await {
        Ok(surveys) => surveys.into_iter().filter(|survey| filter(survey)).collect::<Vec<_>>(),
        Err(error) => {
            prompt.show(&format!("error: {error}"));
            return None;
        }
    };
    let labels: Vec<_> = surveys.iter().map(describe).collect();
    let index = pick_index(prompt, message, &labels, "No surveys to choose from.")?;
    surveys.into_iter().nth(index)
}

/// Asks to pick one of `items` (and tells if there are none).
fn pick_one(prompt: &mut impl Prompt, message: &str, items: Vec<String>, none: &str) -> Option<String> {
    let index = pick_index(prompt, message, &items, none)?;
    items.into_iter().nth(index)
}

fn pick_index(prompt: &mut impl Prompt, message: &str, labels: &[String], none: &str) -> Option<usize> {
    if labels.is_empty() {
        prompt.show(none);
        return None;
    }
    prompt.select(message, labels)
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use prono::MemoryRepo;
    use prono_api::{Submission, samples};

    use super::*;

    /// What the administrator does next.
    enum Reply {
        /// Picks the option containing this text.
        Pick(&'static str),
        Type(&'static str),
        Yes,
        No,
        /// Esc.
        Back,
    }
    use Reply::{Back, No, Pick, Type, Yes};

    /// Answers the questions in order; once the script is out, it goes back
    /// (so a menu always ends).
    struct Script {
        replies: VecDeque<Reply>,
        shown: Vec<String>,
        /// The options of every `select`.
        menus: Vec<Vec<String>>,
    }

    impl Script {
        fn new(replies: impl IntoIterator<Item = Reply>) -> Self {
            Self {
                replies: replies.into_iter().collect(),
                shown: Vec::new(),
                menus: Vec::new(),
            }
        }
    }

    impl Prompt for Script {
        fn select(&mut self, message: &str, options: &[String]) -> Option<usize> {
            self.menus.push(options.to_vec());
            match self.replies.pop_front()? {
                Pick(text) => Some(
                    options
                        .iter()
                        .position(|option| option.contains(text))
                        .unwrap_or_else(|| panic!("no \"{text}\" in {options:?} of \"{message}\"")),
                ),
                Back => None,
                _ => panic!("\"{message}\" got a reply that is no choice"),
            }
        }

        fn text(&mut self, message: &str) -> Option<String> {
            match self.replies.pop_front()? {
                Type(text) => Some(text.into()),
                Back => None,
                _ => panic!("\"{message}\" got a reply that is no text"),
            }
        }

        fn confirm(&mut self, _message: &str) -> bool {
            match self.replies.pop_front() {
                Some(Yes) => true,
                Some(No) | None => false,
                Some(_) => panic!("a confirmation got a reply that is no answer"),
            }
        }

        fn show(&mut self, text: &str) {
            self.shown.push(text.to_string());
        }
    }

    fn repo() -> MemoryRepo {
        MemoryRepo::with_surveys([samples::STARSHIP_JSON, samples::COURSE_JSON])
    }

    async fn navigate(repo: &MemoryRepo, replies: impl IntoIterator<Item = Reply>) -> Script {
        let mut script = Script::new(replies);
        run(repo, &mut script).await;
        script
    }

    #[tokio::test]
    async fn the_main_menu_ends_with_escape() {
        let script = navigate(&repo(), [Back]).await;
        assert_eq!(script.menus, [["Surveys", "Users"]]);
        assert_eq!(script.shown, Vec::<String>::new());
    }

    #[tokio::test]
    async fn menus_return_to_their_parent_and_stay_after_an_action() {
        let script = navigate(
            &repo(),
            [
                Pick("Surveys"),
                Pick("List"),
                // Still in the surveys menu: list again, then go back, then quit.
                Pick("List"),
                Back,
                Back,
            ],
        )
        .await;
        assert_eq!(script.shown.len(), 2);
        assert!(script.shown[0].contains("Starship"), "{:?}", script.shown);
        // Main, surveys, surveys, surveys, main.
        assert_eq!(script.menus.len(), 5);
    }

    #[tokio::test]
    async fn a_survey_file_is_uploaded_by_path() {
        let repo = MemoryRepo::default();
        let path = std::env::temp_dir().join(format!("prono-cli-menu-{}.json", std::process::id()));
        std::fs::write(&path, samples::COURSE_JSON).unwrap();
        // The path is typed as text: leak it into the script for the test.
        let typed: &'static str = Box::leak(path.to_string_lossy().into_owned().into_boxed_str());
        let script = navigate(&repo, [Pick("Surveys"), Pick("Upload"), Type(typed), Back, Back]).await;
        std::fs::remove_file(path).unwrap();

        assert!(script.shown[0].starts_with("Survey uploaded: 2:"), "{:?}", script.shown);
        assert_eq!(prono::survey(&repo, 2).await.unwrap(), samples::course());
    }

    #[tokio::test]
    async fn an_upload_without_a_path_or_with_a_bad_one_changes_nothing() {
        let repo = MemoryRepo::default();
        let script = navigate(
            &repo,
            [
                Pick("Surveys"),
                Pick("Upload"),
                Back, // no path
                Pick("Upload"),
                Type("/nonexistent.json"),
                Back,
                Back,
            ],
        )
        .await;
        assert_eq!(script.shown.len(), 1);
        assert!(script.shown[0].starts_with("error: cannot read"), "{:?}", script.shown);
        assert_eq!(prono::surveys(&repo).await.unwrap(), Vec::<Survey>::new());
    }

    #[tokio::test]
    async fn scores_are_offered_for_scored_surveys_only() {
        let script = navigate(&repo(), [Pick("Surveys"), Pick("scores"), Back, Back, Back]).await;
        assert_eq!(script.shown, ["No surveys to choose from."]);

        let quiz = r#"{"survey_id": 3, "title": "Quiz", "description": "A quiz",
            "questions": [{"question": "Q?", "answer_type": "text", "correct_answer": "a"}]}"#;
        let repo = MemoryRepo::with_surveys([samples::STARSHIP_JSON, quiz]);
        let script = navigate(&repo, [Pick("Surveys"), Pick("scores"), Pick("Quiz"), Back, Back]).await;
        assert_eq!(script.menus[2], ["3: Quiz (1 questions, scored)"]);
        assert_eq!(script.shown, ["Nobody answered survey 3 yet."]);
    }

    #[tokio::test]
    async fn answers_are_shown_for_everybody_or_one_participant() {
        let repo = repo();
        let survey = prono::survey(&repo, 2).await.unwrap();
        repo.set_pass_phrase(2, "rust rocks").await.unwrap();
        repo.allow_user(2, "Ann", "Lee").await.unwrap();
        let submission = Submission::new("Ann Lee", &survey, &samples::course_answers()).with_pass_phrase("rust rocks");
        prono::submit(&repo, 2, "dev-1", &submission).await.unwrap();

        let script = navigate(
            &repo,
            [
                Pick("Surveys"),
                Pick("answers"),
                Pick("Rust course"),
                Pick("Everybody"),
                Pick("answers"),
                Pick("Rust course"),
                Pick("Ann Lee"),
                Back,
                Back,
            ],
        )
        .await;
        assert_eq!(script.menus[3], ["Everybody (a table)", "Ann Lee"]);
        assert!(script.shown[0].contains("Participant | Q1"), "{:?}", script.shown);
        assert_eq!(
            script.shown[1],
            "What is your background: C\nI come to the course!: Of course!\nDo you have any diatery requirements or allergies?: no\nOther requests or questions?: No remarks"
        );
    }

    #[tokio::test]
    async fn answers_of_a_survey_nobody_answered_are_not_offered() {
        let script = navigate(
            &repo(),
            [Pick("Surveys"), Pick("answers"), Pick("Starship"), Back, Back],
        )
        .await;
        assert_eq!(script.shown, ["Nobody answered this survey yet."]);
    }

    #[tokio::test]
    async fn users_are_deleted_after_confirming() {
        let repo = repo();
        repo.save_answers("alice", "dev-1", &[]).await.unwrap();
        repo.save_answers("bob", "dev-2", &[]).await.unwrap();

        // Declined: nothing happens.
        navigate(&repo, [Pick("Users"), Pick("Delete"), Pick("alice"), No, Back, Back]).await;
        assert_eq!(repo.users().await.unwrap(), ["alice", "bob"]);

        let script = navigate(
            &repo,
            [
                Pick("Users"),
                Pick("Delete"),
                Pick("alice"),
                Yes,
                Pick("Show"),
                Back,
                Back,
            ],
        )
        .await;
        assert_eq!(script.shown, ["User 'alice' deleted.", "bob"]);
        assert_eq!(repo.users().await.unwrap(), ["bob"]);
    }

    #[tokio::test]
    async fn deleting_with_no_users_says_so() {
        let script = navigate(&repo(), [Pick("Users"), Pick("Delete"), Back, Back]).await;
        assert_eq!(script.shown, ["No users found."]);
    }

    #[tokio::test]
    async fn the_pass_phrase_is_set_from_the_surveys_menu() {
        let repo = repo();
        let script = navigate(
            &repo,
            [
                Pick("Surveys"),
                Pick("pass phrase"),
                Pick("Starship"),
                Type("open sesame"),
                Back, // the surveys menu
                Back, // main
            ],
        )
        .await;
        assert_eq!(script.shown, ["Pass phrase of survey 1 set. It opens survey(s) 1."]);
        assert_eq!(repo.pass_phrases().await.unwrap(), [(1, "open sesame".to_string())]);
    }

    #[tokio::test]
    async fn cancelling_or_a_blank_pass_phrase_changes_nothing() {
        let repo = repo();
        let script = navigate(
            &repo,
            [
                Pick("Surveys"),
                Pick("pass phrase"),
                Pick("Starship"),
                Back, // no pass phrase
                Pick("pass phrase"),
                Back, // no survey
                Pick("pass phrase"),
                Pick("Starship"),
                Type("  "),
                Back,
                Back,
            ],
        )
        .await;
        assert_eq!(script.shown.len(), 1);
        assert!(script.shown[0].starts_with("error:"), "{:?}", script.shown);
        assert_eq!(repo.pass_phrases().await.unwrap(), Vec::<(u64, String)>::new());
    }

    #[tokio::test]
    async fn a_pass_phrase_needs_a_survey_to_choose() {
        let script = navigate(
            &MemoryRepo::default(),
            [Pick("Surveys"), Pick("pass phrase"), Back, Back],
        )
        .await;
        assert_eq!(script.shown, ["No surveys to choose from."]);
    }
}
