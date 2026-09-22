//! Authoritative focus-session domain rules.

pub mod focus_session;
pub mod plant_bundle;
#[allow(dead_code)] // M3 produces a replayable domain value; M5 first consumes it.
pub mod plant_generator;
