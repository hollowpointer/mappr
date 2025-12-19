//! # Application Services
//!
//! This module contains the "Use Cases" of the application.
//!
//! ## Role in Hexagonal Architecture
//! Application Services sit at the center of the hexagon (along with the Domain).
//! They are the entry points for all business logic operation.
//!
//! * **Orchestration**: They coordinate the interaction between the Domain layer (pure logic) and the Ports (infrastructure).
//! * **Transaction Scripts**: Each service method typically corresponds to a specific user intent or command (e.g., "Scan Network", "Get System Info").
//! * **Agnostic**: They do not know *how* underlying actions are performed (e.g., network scanning implementation), only *that* they are performed via Ports.
//!
//! ## Available Services
//! * [`discovery::DiscoveryService`]: Orchestrates network scanning and host discovery.
//! * [`info::InfoService`]: Gathers information about the local system (interfaces, services).

pub mod discovery;
pub mod info;
