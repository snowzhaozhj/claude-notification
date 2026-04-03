// claude-notify-core: shared types and utilities

pub mod analyzer;
pub mod config;
pub mod decision;
pub mod dedup;
pub mod error;
pub mod hook;
pub mod priority;
pub mod state;
pub mod summary;
pub mod suppression;
pub mod types;

pub use error::{NotifyError, Result};
pub use types::{Channel, ClickAction, Decision, Notification, NotifyEvent, Priority, Status};
