use clap::{Parser, Subcommand};
use prono::ReadConfig;
use prono::repo::{Db, Users};

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
async fn main() {
    env_logger::init();
    let cli = Cli::parse();

    let config_reader = prono::factory::create_config_reader();
    let default_config_path = config_reader.default_config_path();
    let db_config: prono_db::Config = config_reader.read(default_config_path).db.into();
    let db = prono_db::MysqlDb::init(db_config)
        .await
        .expect("Failed to initialize database");

    match cli.command {
        Commands::Users { action } => match action {
            UserAction::Show => {
                let users = db.all_users().await.expect("Failed to fetch users");
                if users.is_empty() {
                    println!("No users found.");
                } else {
                    for user in &users {
                        println!("{user}");
                    }
                }
            }
            UserAction::Delete { name } => {
                db.delete_user(&name).await.expect("Failed to delete user");
                println!("User '{name}' deleted.");
            }
        },
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
            Commands::Users { .. } => panic!("Expected Users Delete command"),
        }
    }

    #[test]
    fn parse_missing_subcommand_fails() {
        let result = Cli::try_parse_from(["prono-cli"]);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().kind(),
            ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
        );
    }

    #[test]
    fn parse_delete_missing_name_fails() {
        let result = Cli::try_parse_from(["prono-cli", "users", "delete"]);
        assert!(result.is_err());
    }
}
