// claude-notify-core: shared types and utilities

pub mod error;
pub mod types;

pub use error::{NotifyError, Result};
pub use types::{
    Channel, ClickAction, Decision, Notification, NotifyEvent, Priority, Status,
};
