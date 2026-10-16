//! Pure capture models and bounded, independently readable event history.

pub mod broker;
pub mod clicks;
pub mod commands;
pub mod context;
pub mod dossier;

#[cfg(test)]
mod command_tests;
