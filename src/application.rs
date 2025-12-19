//! # Application Layer (Service Layer)
//!
//! This layer orchestrates the business logic user cases.
//!
//! ## Purpose
//! It acts as the "API" for the domain. It does not contain complex business rules (that belongs in `domain`),
//! but rather:
//! 1. Receives a command from an Inbound Adapter.
//! 2. Validates inputs.
//! 3. Calls the appropriate Domain entities or Outbound Ports.
//! 4. Returns results to the adapter.
//!
//! ## Contents
//! * **[`services`]**: Grouped by feature/context (e.g., `discovery`, `info`).

pub mod services;
