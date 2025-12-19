//! # Domain Models
//!
//! This module contains the core data structures and types that represent the
//! business domain of the application.
//!
//! ## Core Entities
//! * [`host::Host`]: The primary entity representing a discovered network device.
//! * [`target::Target`]: Represents the input target for a scan (IP, Range, LAN).
//!
//! ## Value Objects
//! * [`range::Ipv4Range`]: Represents an inclusive range of IPv4 addresses.
//! * [`local_system::IpServiceGroup`]: Represents local system information (open ports, firewall).
//!
//! ## Design Principles
//! * **Rich Models**: Models should contain logic for validation, parsing, and data manipulation.
//! * **Immutability**: Where possible, prefer immutable state or builder patterns (`with_mac`).
//! * **Portability**: Models are used across all layers (Ports, Adapters, Application).

pub mod host;
pub mod target;
pub mod range;
pub mod local_system;
