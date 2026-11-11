//! The interactive mode: menus to navigate instead of commands to remember.
//! Every menu builds a [`Commands`] and hands it to [`admin::run`], so it does
//! exactly what the command line does. The questions go through a [`Prompt`],
//! which the terminal implements and the tests script.

use std::path::PathBuf;

use prono::Repo;
use prono_api::{Access, Survey};

use crate::admin::{self, describe};
use crate::commands::{AccessAction, Commands, SurveyAction, UserAction};

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
    while let Some(choice) = pick(prompt, "Prono admin (Esc quits)", &["Surveys", "Users", "Access"]) {
        match choice {
            0 => surveys_menu(repo, prompt).await,
            1 => users_menu(repo, prompt).await,
            _ => access_menu(repo, prompt).await,
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
        "Show the scores of a survey",
    ];
    while let Some(choice) = pick(prompt, "Surveys (Esc goes back)", &options) {
        let action = match choice {
            0 => SurveyAction::List,
            1 => {
                let Some(file) = prompt.text("Path of the survey JSON file") else {
                    continue;
                };
                SurveyAction::Upload {
                    file: PathBuf::from(file.trim()),
                }
            }
            _ => {
                let Some(survey) = pick_survey(repo, prompt, "Scores of which survey?", |survey| survey.scored).await
                else {
                    continue;
                };
                SurveyAction::Scores { survey_id: survey.id }
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

async fn access_menu(repo: &dyn Repo, prompt: &mut impl Prompt) {
    while let Some(survey) = pick_survey(repo, prompt, "Access to which survey? (Esc goes back)", |_| true).await {
        match survey.access {
            Access::NameList => name_list_menu(repo, prompt, &survey).await,
            Access::PassPhrase => pass_phrase_menu(repo, prompt, &survey).await,
        }
    }
}

async fn name_list_menu(repo: &dyn Repo, prompt: &mut impl Prompt, survey: &Survey) {
    let survey_id = survey.id;
    let title = format!("Name list of {} (Esc goes back)", describe(survey));
    let options = ["Show the name list", "Allow a participant", "Disallow a participant"];
    while let Some(choice) = pick(prompt, &title, &options) {
        let action = match choice {
            0 => AccessAction::Show { survey_id },
            1 => {
                let Some(first_name) = prompt.text("First name") else {
                    continue;
                };
                let Some(last_name) = prompt.text("Last name") else {
                    continue;
                };
                AccessAction::Allow {
                    survey_id,
                    first_name,
                    last_name,
                }
            }
            _ => {
                let names = match repo.allowed_users(survey_id).await {
                    Ok(names) => names,
                    Err(error) => {
                        prompt.show(&format!("error: {error}"));
                        continue;
                    }
                };
                let labels: Vec<_> = names.iter().map(|(first, last)| format!("{first} {last}")).collect();
                let Some(index) = pick_index(prompt, "Disallow whom?", &labels, "Nobody is on the list.") else {
                    continue;
                };
                let (first_name, last_name) = names[index].clone();
                AccessAction::Disallow {
                    survey_id,
                    first_name,
                    last_name,
                }
            }
        };
        execute(repo, prompt, Commands::Access { action }).await;
    }
}

async fn pass_phrase_menu(repo: &dyn Repo, prompt: &mut impl Prompt, survey: &Survey) {
    let title = format!("Pass phrase of {} (Esc goes back)", describe(survey));
    while pick(prompt, &title, &["Set the pass phrase"]).is_some() {
        let Some(pass_phrase) = prompt.text("The new pass phrase") else {
            continue;
        };
        let action = AccessAction::PassPhrase {
            survey_id: survey.id,
            pass_phrase,
        };
        execute(repo, prompt, Commands::Access { action }).await;
    }
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
    use prono_api::samples;

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
        assert_eq!(script.menus, [["Surveys", "Users", "Access"]]);
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
        assert!(script.shown[0].contains("SpaceX"), "{:?}", script.shown);
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

        let quiz = r#"{"survey_id": 3, "description": "Quiz", "access": "name_list",
            "questions": [{"question": "Q?", "answer_type": "text", "correct_answer": "a"}]}"#;
        let repo = MemoryRepo::with_surveys([samples::STARSHIP_JSON, quiz]);
        let script = navigate(&repo, [Pick("Surveys"), Pick("scores"), Pick("Quiz"), Back, Back]).await;
        assert_eq!(script.menus[2], ["3: Quiz (name list, 1 questions, scored)"]);
        assert_eq!(script.shown, ["Nobody answered survey 3 yet."]);
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
    async fn the_name_list_of_a_survey_is_managed_in_its_menu() {
        let repo = repo();
        let script = navigate(
            &repo,
            [
                Pick("Access"),
                Pick("name list"),
                Pick("Allow"),
                Type("Ada"),
                Type("Lovelace"),
                Pick("Show"),
                Pick("Disallow"),
                Pick("Ada Lovelace"),
                Back, // the name list menu
                Back, // the survey list
                Back, // main
            ],
        )
        .await;
        assert_eq!(
            script.shown,
            [
                "'Ada Lovelace' may answer survey 2.",
                "Ada Lovelace",
                "'Ada Lovelace' may no longer answer survey 2."
            ]
        );
        assert_eq!(repo.allowed_users(2).await.unwrap(), Vec::<(String, String)>::new());
    }

    #[tokio::test]
    async fn only_the_actions_that_fit_the_access_are_offered() {
        let script = navigate(
            &repo(),
            [
                Pick("Access"),
                Pick("pass phrase"),
                Back,
                Pick("name list"),
                Back,
                Back,
                Back,
            ],
        )
        .await;
        assert_eq!(script.menus[2], ["Set the pass phrase"]);
        assert_eq!(
            script.menus[4],
            ["Show the name list", "Allow a participant", "Disallow a participant"]
        );
    }

    #[tokio::test]
    async fn the_pass_phrase_is_set_from_its_menu() {
        let repo = repo();
        let script = navigate(
            &repo,
            [
                Pick("Access"),
                Pick("pass phrase"),
                Pick("Set"),
                Type("open sesame"),
                Back,
                Back,
                Back,
            ],
        )
        .await;
        assert_eq!(script.shown, ["Pass phrase of survey 1 set."]);
        assert_eq!(repo.pass_phrase(1).await.unwrap().as_deref(), Some("open sesame"));
    }

    #[tokio::test]
    async fn cancelling_a_question_returns_to_the_menu() {
        let repo = repo();
        let script = navigate(
            &repo,
            [
                Pick("Access"),
                Pick("name list"),
                Pick("Allow"),
                Type("Ada"),
                Back,             // no last name
                Pick("Disallow"), // nobody on the list
                Back,
                Back,
                Back,
            ],
        )
        .await;
        assert_eq!(script.shown, ["Nobody is on the list."]);
        assert_eq!(repo.allowed_users(2).await.unwrap(), Vec::<(String, String)>::new());
    }

    #[tokio::test]
    async fn access_with_no_surveys_says_so() {
        let script = navigate(&MemoryRepo::default(), [Pick("Access"), Back]).await;
        assert_eq!(script.shown, ["No surveys to choose from."]);
    }

    #[tokio::test]
    async fn failures_are_shown_and_the_menu_goes_on() {
        let repo = repo();
        let script = navigate(
            &repo,
            [
                Pick("Access"),
                Pick("name list"),
                Pick("Allow"),
                Type("Ada"),
                Type("  "),
                Pick("Show"),
                Back,
                Back,
                Back,
            ],
        )
        .await;
        assert!(script.shown[0].starts_with("error:"), "{:?}", script.shown);
        assert_eq!(script.shown[1], "Nobody is on the list of survey 2.");
    }
}
