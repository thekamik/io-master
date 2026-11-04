//! `panels/views/`: only the commands the page calls, a folder per panel, and what they
//! share: their error, the open database, the progress, a new recovery code.

pub mod account;
pub mod agents;
pub mod chat;
pub mod error;
pub mod evaluation;
pub mod evaluation_plan;
pub mod opened;
pub mod program_start;
pub mod progress;
pub mod recovery_code;
pub mod reports;
pub mod settings;
