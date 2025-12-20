//! # Mappr Codebase
//!
//! `mappr` is a network analysis tool designed with **Hexagonal Architecture**.
//!
//! ## Architecture Overview
//! The codebase is organized into layers to separate concerns and ensure maintainability:
//!
//! * **[`domain`]**: The core business logic and models. Pure Rust, no external IO dependencies.
//!     * *Center of the Hexagon*.
//! * **[`application`]**: Application services and use cases. Orchestrates the Domain and Ports.
//!     * *Application Layer*.
//! * **[`ports`]**: Traits defining interactions between the Application and the outside world.
//!     * *Boundaries of the Hexagon*.
//! * **[`adapters`]**: Concrete implementations of Ports (CLI, Network, OS).
//!     * *Outside the Hexagon*.
//!
//! ## Helper Modules
//! * **[`utils`]**: Shared utilities.

pub mod adapters; 
pub mod application; 
pub mod domain;
pub mod ports; 
pub mod utils;


