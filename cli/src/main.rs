mod admin;
mod commands;

use std::process::ExitCode;

use clap::Parser;
use commands::Commands;
use prono::{DbConfig, MysqlRepo};

#[derive(Debug, Parser)]
#[command(name = "prono-cli", about = "Prono administration")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
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

    println!("{}", admin::run(&repo, cli.command).await?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use commands::{AccessAction, SurveyAction, UserAction};

    use super::*;

    fn parse(args: &[&str]) -> Result<Commands, clap::Error> {
        Cli::try_parse_from(std::iter::once("prono-cli").chain(args.iter().copied())).map(|cli| cli.command)
    }

    #[test]
    fn a_command_is_required() {
        assert!(parse(&[]).is_err());
    }

    #[test]
    fn parse_users() {
        assert_eq!(
            parse(&["users", "show"]).unwrap(),
            Commands::Users {
                action: UserAction::Show
            }
        );
        assert_eq!(
            parse(&["users", "delete", "alice"]).unwrap(),
            Commands::Users {
                action: UserAction::Delete { name: "alice".into() }
            }
        );
        assert!(parse(&["users", "delete"]).is_err());
    }

    #[test]
    fn parse_access_allow() {
        assert_eq!(
            parse(&["access", "allow", "2", "Ada", "Lovelace"]).unwrap(),
            Commands::Access {
                action: AccessAction::Allow {
                    survey_id: 2,
                    first_name: "Ada".into(),
                    last_name: "Lovelace".into()
                }
            }
        );
        assert!(parse(&["access", "allow", "2", "Ada"]).is_err());
        assert!(parse(&["access", "allow", "x", "Ada", "L"]).is_err());
    }

    #[test]
    fn parse_access_pass_phrase() {
        assert!(matches!(
            parse(&["access", "pass-phrase", "3", "open sesame"]).unwrap(),
            Commands::Access {
                action: AccessAction::PassPhrase { survey_id: 3, .. }
            }
        ));
    }

    #[test]
    fn parse_surveys() {
        assert_eq!(
            parse(&["surveys", "list"]).unwrap(),
            Commands::Surveys {
                action: SurveyAction::List
            }
        );
        assert_eq!(
            parse(&["surveys", "upload", "a/b.json"]).unwrap(),
            Commands::Surveys {
                action: SurveyAction::Upload {
                    file: "a/b.json".into()
                }
            }
        );
        assert_eq!(
            parse(&["surveys", "scores", "3"]).unwrap(),
            Commands::Surveys {
                action: SurveyAction::Scores { survey_id: 3 }
            }
        );
        assert!(parse(&["surveys", "upload"]).is_err());
    }
}
