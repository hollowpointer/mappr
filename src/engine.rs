//! # Network Engine (Low-Level)
//!
//! This module contains the low-level implementations for network interactions.
//!
//! ## Architectural Warning
//! ⚠️ **Implementations here should generally be hidden behind Adapters.**
//! Use the entities in `src/adapters/outbound/network` to expose this functionality
//! to the rest of the application.
//!
//! ## Contents
//! * `tcp_connect`: Raw TCP connection logic.
//! * `ip`: IP packet constraints and utilities.
//! * `datalink`: Layer 2 interface management.
//! * `scanner`: Core algorithms for scanning.

pub mod datalink;
pub mod protocol;
pub mod runner;
pub mod scanner;
pub mod sender;
pub mod tcp_connect;
pub mod transport;
pub mod utils;
pub mod models;
pub mod ip;
