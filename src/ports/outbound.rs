//! # Outbound Ports (Driven Actors)
//!
//! This module defines the contracts (traits) for interactions *initiated by the application*
//! towards the external world (Infrastructure).
//!
//! ## What belongs here?
//! * **Repositories**: Interfaces for data access (database, file system).
//! * **Gateways**: Interfaces for external services (API clients, message buses).
//! * **Adapters**: Interfaces for low-level system operations (networking, OS calls).
//!
//! ## Rules
//! 1. All items here must be `traits`.
//! 2. No concrete implementations allowed.
//! 3. Using `domain` models in method signatures is allowed and encouraged.
//! 4. These traits are implemented in `adapters/outbound`.
pub mod vendor_repository;
pub mod system_repository;
pub mod network_scanner;
