use std::process::ExitCode;

use clap::{Parser, Subcommand};
use prono::{DbConfig, MysqlRepo, Repo};
use prono_api::{Access, check_name_parts};

#[derive(Debug, Parser)]
#[command(name = "prono-cli", about = "Prono database management CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Manage users
    Users {
        #[command(subcommand)]
        action: UserAction,
    },
    /// Manage who may answer a survey
    Access {
        #[command(subcommand)]
        action: AccessAction,
    },
}

#[derive(Debug, Subcommand)]
enum UserAction {
    /// Show all users
    Show,
    /// Delete a user and all their answers
    Delete {
        /// Username to delete
        name: String,
    },
}

#[derive(Debug, Subcommand)]
enum AccessAction {
    /// Put a participant on the name list of a survey (`"access": "name_list"`)
    Allow {
        survey_id: u64,
        first_name: String,
        last_name: String,
    },
    /// Take a participant off the name list of a survey
    Disallow {
        survey_id: u64,
        first_name: String,
        last_name: String,
    },
    /// Show the name list of a survey
    Show { survey_id: u64 },
    /// Set the pass phrase of a survey (`"access": "pass_phrase"`)
    PassPhrase { survey_id: u64, pass_phrase: String },
}

#[tokio::main]
async fn main() -> ExitCode {
    env_logger::init();
    let cli = Cli::parse();
    match run(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

async fn run(cli: Cli) -> Result<(), String> {
    let config = DbConfig::load()?;
    let repo = MysqlRepo::connect(&config).await.map_err(|e| e.to_string())?;

    match cli.command {
        Commands::Users { action } => match action {
            UserAction::Show => {
                let users = repo.users().await.map_err(|e| e.to_string())?;
                if users.is_empty() {
                    println!("No users found.");
                }
                for user in &users {
                    println!("{user}");
                }
            }
            UserAction::Delete { name } => {
                repo.delete_user(&name).await.map_err(|e| e.to_string())?;
                println!("User '{name}' deleted.");
            }
        },
        Commands::Access { action } => access(&repo, action).await?,
    }
    Ok(())
}

async fn access(repo: &impl Repo, action: AccessAction) -> Result<(), String> {
    match action {
        AccessAction::Allow {
            survey_id,
            first_name,
            last_name,
        } => {
            require_access(survey_id, Access::NameList)?;
            let name = check_name_parts(&first_name, &last_name).map_err(|e| e.to_string())?;
            repo.allow_user(survey_id, first_name.trim(), last_name.trim())
                .await
                .map_err(|e| e.to_string())?;
            println!("'{name}' may answer survey {survey_id}.");
        }
        AccessAction::Disallow {
            survey_id,
            first_name,
            last_name,
        } => {
            require_access(survey_id, Access::NameList)?;
            repo.disallow_user(survey_id, first_name.trim(), last_name.trim())
                .await
                .map_err(|e| e.to_string())?;
            println!(
                "'{} {}' may no longer answer survey {survey_id}.",
                first_name.trim(),
                last_name.trim()
            );
        }
        AccessAction::Show { survey_id } => {
            require_access(survey_id, Access::NameList)?;
            let names = repo.allowed_users(survey_id).await.map_err(|e| e.to_string())?;
            if names.is_empty() {
                println!("Nobody is on the list of survey {survey_id}.");
            }
            for (first_name, last_name) in names {
                println!("{first_name} {last_name}");
            }
        }
        AccessAction::PassPhrase { survey_id, pass_phrase } => {
            require_access(survey_id, Access::PassPhrase)?;
            let pass_phrase = pass_phrase.trim();
            if pass_phrase.is_empty() {
                return Err("the pass phrase must not be blank".into());
            }
            repo.set_pass_phrase(survey_id, pass_phrase)
                .await
                .map_err(|e| e.to_string())?;
            println!("Pass phrase of survey {survey_id} set.");
        }
    }
    Ok(())
}

/// Fails unless survey `survey_id` exists and uses `access`: a list or pass
/// phrase for a survey that does not check it would silently do nothing.
fn require_access(survey_id: u64, access: Access) -> Result<(), String> {
    let survey = prono_api::survey(survey_id).ok_or_else(|| format!("unknown survey {survey_id}"))?;
    if survey.access == access {
        Ok(())
    } else {
        Err(format!(
            "survey {survey_id} has access {:?}, not {access:?} (see `api/surveys/`)",
            survey.access
        ))
    }
}

#[cfg(test)]
mod tests {
    use clap::error::ErrorKind;

    use super::*;

    #[test]
    fn parse_users_show() {
        let cli = Cli::try_parse_from(["prono-cli", "users", "show"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Users {
                action: UserAction::Show
            }
        ));
    }

    #[test]
    fn parse_users_delete() {
        let cli = Cli::try_parse_from(["prono-cli", "users", "delete", "alice"]).unwrap();
        match cli.command {
            Commands::Users {
                action: UserAction::Delete { name },
            } => assert_eq!(name, "alice"),
            _ => panic!("Expected Users Delete command"),
        }
    }

    #[test]
    fn parse_missing_subcommand_fails() {
        let result = Cli::try_parse_from(["prono-cli"]);
        assert_eq!(
            result.unwrap_err().kind(),
            ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
        );
    }

    #[test]
    fn parse_delete_missing_name_fails() {
        assert!(Cli::try_parse_from(["prono-cli", "users", "delete"]).is_err());
    }

    #[test]
    fn parse_access_allow() {
        let cli = Cli::try_parse_from(["prono-cli", "access", "allow", "2", "Ada", "Lovelace"]).unwrap();
        match cli.command {
            Commands::Access {
                action:
                    AccessAction::Allow {
                        survey_id,
                        first_name,
                        last_name,
                    },
            } => assert_eq!(
                (survey_id, first_name.as_str(), last_name.as_str()),
                (2, "Ada", "Lovelace")
            ),
            _ => panic!("Expected Access Allow command"),
        }
        assert!(Cli::try_parse_from(["prono-cli", "access", "allow", "2", "Ada"]).is_err());
        assert!(Cli::try_parse_from(["prono-cli", "access", "allow", "x", "Ada", "L"]).is_err());
    }

    #[test]
    fn parse_access_pass_phrase() {
        let cli = Cli::try_parse_from(["prono-cli", "access", "pass-phrase", "3", "open sesame"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Access {
                action: AccessAction::PassPhrase { survey_id: 3, .. }
            }
        ));
    }

    #[test]
    fn access_commands_only_fit_surveys_using_that_access() {
        assert_eq!(require_access(2, Access::NameList), Ok(()));
        assert!(require_access(1, Access::NameList).unwrap_err().contains("Open"));
        assert!(require_access(2, Access::PassPhrase).is_err());
        assert!(
            require_access(999, Access::NameList)
                .unwrap_err()
                .contains("unknown survey")
        );
    }
}
