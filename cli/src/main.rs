mod admin;
mod commands;
mod menu;
mod terminal;

use std::io::IsTerminal as _;
use std::process::ExitCode;

use clap::Parser;
use commands::Commands;
use prono::{DbConfig, MysqlRepo};

#[derive(Debug, Parser)]
#[command(
    name = "prono-cli",
    about = "Prono administration. Without a command it opens an interactive menu."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
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
    if cli.command.is_none() && !std::io::stdin().is_terminal() {
        return Err("no command given, and there is no terminal for the interactive menu (see --help)".into());
    }
    let config = DbConfig::load()?;
    let repo = MysqlRepo::connect(&config).await.map_err(|e| e.to_string())?;

    match cli.command {
        Some(command) => println!("{}", admin::run(&repo, command).await?),
        None => menu::run(&repo, &mut terminal::Terminal).await,
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use commands::{AccessAction, SurveyAction, UserAction};

    use super::*;

    fn parse(args: &[&str]) -> Result<Option<Commands>, clap::Error> {
        Cli::try_parse_from(std::iter::once("prono-cli").chain(args.iter().copied())).map(|cli| cli.command)
    }

    #[test]
    fn no_command_means_the_interactive_menu() {
        assert_eq!(parse(&[]).unwrap(), None);
    }

    #[test]
    fn parse_users() {
        assert_eq!(
            parse(&["users", "show"]).unwrap(),
            Some(Commands::Users {
                action: UserAction::Show
            })
        );
        assert_eq!(
            parse(&["users", "delete", "alice"]).unwrap(),
            Some(Commands::Users {
                action: UserAction::Delete { name: "alice".into() }
            })
        );
        assert!(parse(&["users", "delete"]).is_err());
    }

    #[test]
    fn parse_access_allow() {
        assert_eq!(
            parse(&["access", "allow", "2", "Ada", "Lovelace"]).unwrap(),
            Some(Commands::Access {
                action: AccessAction::Allow {
                    survey_id: 2,
                    first_name: "Ada".into(),
                    last_name: "Lovelace".into()
                }
            })
        );
        assert!(parse(&["access", "allow", "2", "Ada"]).is_err());
        assert!(parse(&["access", "allow", "x", "Ada", "L"]).is_err());
    }

    #[test]
    fn parse_access_pass_phrase() {
        assert!(matches!(
            parse(&["access", "pass-phrase", "3", "open sesame"]).unwrap(),
            Some(Commands::Access {
                action: AccessAction::PassPhrase { survey_id: 3, .. }
            })
        ));
    }

    #[test]
    fn parse_surveys() {
        assert_eq!(
            parse(&["surveys", "list"]).unwrap(),
            Some(Commands::Surveys {
                action: SurveyAction::List
            })
        );
        assert_eq!(
            parse(&["surveys", "upload", "a/b.json"]).unwrap(),
            Some(Commands::Surveys {
                action: SurveyAction::Upload {
                    file: "a/b.json".into()
                }
            })
        );
        assert_eq!(
            parse(&["surveys", "scores", "3"]).unwrap(),
            Some(Commands::Surveys {
                action: SurveyAction::Scores { survey_id: 3 }
            })
        );
        assert!(parse(&["surveys", "upload"]).is_err());
    }
}
