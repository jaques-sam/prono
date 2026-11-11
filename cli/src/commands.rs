//! What the administrator can ask for, as typed on the command line or
//! picked in the interactive menu (see [`crate::menu`]).

use std::path::PathBuf;

use clap::Subcommand;

#[derive(Debug, PartialEq, Eq, Subcommand)]
pub enum Commands {
    /// Manage surveys
    Surveys {
        #[command(subcommand)]
        action: SurveyAction,
    },
    /// Manage users
    Users {
        #[command(subcommand)]
        action: UserAction,
    },
}

#[derive(Debug, PartialEq, Eq, Subcommand)]
pub enum SurveyAction {
    /// Show all surveys
    List,
    /// Upload a survey definition (JSON, see `api/src/definition.rs`), replacing the survey with
    /// the same id
    Upload {
        /// The definition file
        file: PathBuf,
    },
    /// Show who got how many answers right in a scored survey
    Scores { survey_id: u64 },
    /// Set the pass phrase of a survey, which is the only way to see and answer it. Surveys that
    /// share a pass phrase are all shown to whoever enters it
    PassPhrase { survey_id: u64, pass_phrase: String },
    /// Show the answers to a survey: a table of all participants, or those of one
    Answers {
        survey_id: u64,
        /// Only show the answers of this participant
        #[arg(long)]
        user: Option<String>,
    },
}

#[derive(Debug, PartialEq, Eq, Subcommand)]
pub enum UserAction {
    /// Show all users
    Show,
    /// Delete a user and all their answers
    Delete {
        /// Username to delete
        name: String,
    },
}
