//! Use cases. Orchestrates the domain aggregates, persistence and the OS
//! blocking ports. Contains no business rules of its own.

mod focus_app;
mod mapping;
pub mod ports;

pub use focus_app::{AppError, Dependencies, FocusApp};
