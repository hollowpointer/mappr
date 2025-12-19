//! # Inbound Ports (Driving Actors)
//!
//! This module defines the contracts (traits) for interactions *initiated by external actors*
//! towards the application (Driving the App).
//!
//! ## What belongs here?
//! * **Use Cases**: High-level abstract Interfaces for user actions (e.g., `ScanNetwork`, `GetInfo`).
//!
//! ## Current State
//! Currently, `mappr` uses a simplified approach where the CLI adapters (in `src/adapters/inbound/cli`)
//! instantiate and call `Application Services` directly. This is acceptable for a CLI tool
//! where the "Driver" is always the user via terminal.
//!
//! In a more complex setup (e.g., adding a REST API), we would define traits here like:
//! ```rust
//! use mappr::domain::models::target::Target;
//! use mappr::domain::models::host::Host;
//!
//! pub trait DiscoveryUseCase {
//!     fn perform_discovery(&self, target: Target) -> anyhow::Result<Vec<Host>>;
//! }
//! ```
