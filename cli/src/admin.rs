//! Carries out a [`Commands`] against the repository. The command line and
//! the interactive menu both end up here, so they cannot disagree.

use prono::Repo;
use prono_api::{Access, Survey, check_name_parts};

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
    }
}

/// `1: SpaceX survey (pass phrase, 4 questions)`
#[must_use]
pub fn describe(survey: &Survey) -> String {
    let access = match survey.access {
        Access::PassPhrase => "pass phrase",
        Access::NameList => "name list",
    };
    format!(
        "{}: {} ({access}, {} questions)",
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
            "1: SpaceX Starship achievements survey (pass phrase, 4 questions)",
            "2: You were chosen to follow a splendid course... (name list, 2 questions)",
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
