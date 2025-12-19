use std::net::IpAddr;
use crate::domain::models::range::Ipv4Range;

#[derive(Clone, Debug)]
pub enum Target {
    LAN,
    Host { target_addr: IpAddr },
    Range { ipv4_range: Ipv4Range },
    VPN,
}

// Logic for parsing can stay here or be in a service/util, but for now we keep it with data
// Ideally parsing strings is a "Adapter" concern (converting CLI string -> Domain Object)
// But FromStr is a standard trait.
// Let's copy the struct first. 
// Note: `crate::adapters::outbound::network::range::Ipv4Range` is currently in `src/net`.
// `src/net` will move to `adapters`. Domain should not depend on adapters.
// Refactoring `Ipv4Range` to `domain/models/range.rs` is also needed!
// This demonstrates the ripple effect of "Clean Architecture".

// Plan:
// 1. Move `Ipv4Range` to `domain/models/range.rs`.
// 2. Move `Target` to `domain/models/target.rs` (depending on `range`).
