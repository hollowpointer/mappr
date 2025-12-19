//! # Ports Layer (Boundaries)
//!
//! Defines the interfaces (traits) that isolate the core application from the infrastructure.
//!
//! ## Types of Ports
//! * **[`inbound`]** (Primary/Driving): APIs exposed *by* the application (e.g., `DriveTheApp`).
//! * **[`outbound`]** (Secondary/Driven): APIs required *by* the application (e.g., `SaveData`, `ScanNetwork`).
//!
//! ## Dependency Rule
//! * The Application depends on these Ports.
//! * The Adapters implement these Ports.
//! * This inverts the control flow, keeping the core isolated.

pub mod outbound;
pub mod inbound;
