use anyhow::{Context, Result};
use dns_parser::{Packet, RData};
use std::{collections::HashSet, net::IpAddr};

#[derive(Debug, Default)]
pub struct MdnsMetadata {
    pub hostname: Option<String>,
    pub ips: HashSet<IpAddr>,
}

pub fn extract_resource(data: &[u8]) -> Result<MdnsMetadata> {
    let packet = Packet::parse(data).context("Failed to parse mDNS packet")?;
    let mut metadata = MdnsMetadata::default();

    for record in packet.answers.iter().chain(packet.additional.iter()) {
        match &record.data {
            // 1. Hostname (PTR)
            RData::PTR(ptr) => {
                let name = ptr.0.to_string();
                if !name.ends_with(".arpa") {
                    metadata.hostname = Some(name);
                }
            }
            RData::A(a) => {
                metadata.ips.insert(IpAddr::V4(a.0));
            }
            RData::AAAA(aaaa) => {
                metadata.ips.insert(IpAddr::V6(aaaa.0));
            }
            _ => {}
        }
    }

    Ok(metadata)
}
