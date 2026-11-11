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
    /// Manage who may answer a survey
    Access {
        #[command(subcommand)]
        action: AccessAction,
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

#[derive(Debug, PartialEq, Eq, Subcommand)]
pub enum AccessAction {
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
