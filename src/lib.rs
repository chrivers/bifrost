#![warn(
    clippy::all,
    clippy::correctness,
    clippy::pedantic,
    clippy::cargo,
    clippy::nursery,
    clippy::perf,
    clippy::style
)]
#![allow(
    clippy::cargo_common_metadata,
    clippy::multiple_crate_versions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions
)]

pub mod error;
pub mod hue;
pub mod z2m;

pub mod config;
pub mod mdns;
pub mod resource;
pub mod routes;
pub mod state;
pub mod types;

#[cfg(feature = "mqtt")]
pub mod mqtt;
