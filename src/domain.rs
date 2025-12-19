//! # Domain Layer (Core)
//!
//! The heart of the application. Contains the business logic and models.
//!
//! ## Characteristics
//! * **Pure Rust**: No external dependencies (no IO, no HTTP, no system calls).
//! * **Stability**: Changes here should be rare and driven by business requirements, not technology changes.
//! * **Independence**: Does not know about Ports, Adapters, or the Application layer.
//!
//! ## Contents
//! * **[`models`]**: The entities and value objects of the system.

pub mod models;
