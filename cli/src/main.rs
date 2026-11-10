use std::process::ExitCode;

use clap::{Parser, Subcommand};
use prono::{DbConfig, MysqlRepo, Repo};

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
    }
    Ok(())
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
            Commands::Users { .. } => panic!("Expected Users Delete command"),
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
}
