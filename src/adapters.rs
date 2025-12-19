//! # Adapters Layer (Infrastructure)
//!
//! This layer contains the concrete implementations of the [`crate::ports`].
//! It interacts directly with the outside world (Network, OS, User).
//!
//! ## Architecture
//! In Hexagonal Architecture, Adapters matches the "driving" and "driven" sides:
//!
//! * **[`inbound`]** (Driving): Adapters that *drive* the application (e.g., CLI, REST API).
//! * **[`outbound`]** (Driven): Adapters that *are driven by* the application (e.g., Network Scanner, System Repo).
//!
//! ## Rules
//! * Adapters **MUST** depend on `ports` and `domain`.
//! * Adapters **MUST NOT** depend on `application` logic directly (circular dependency), though Inbound adapters call Application Services.

pub mod inbound;
pub mod outbound;
