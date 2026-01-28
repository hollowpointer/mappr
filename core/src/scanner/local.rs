//! A **local area network (LAN)** scanner.
//!
//! Primarily used for discovering and scanning hosts on the same physical network,
//! using protocols like ARP, NDP, and ICMP for discovery and TCP/UDP for port scanning.
//!
//! This scanner requires **root privileges** to construct and intercept raw
//! Layer 2 packets via the operating system's network sockets.

use std::{
    collections::HashMap,
    net::IpAddr,
    sync::atomic::Ordering,
    time::{Duration, Instant},
};

use anyhow::{anyhow, bail, ensure};
use pnet::{
    datalink::NetworkInterface,
    packet::{
        Packet,
        arp::ArpPacket,
        ethernet::{EtherTypes, EthernetPacket},
    },
    util::MacAddr,
};

use mappr_common::{
    error,
    network::{host::Host, range::IpCollection, target::IS_LAN_SCAN},
    sender::{PacketType, SenderConfig},
    success,
    utils::timing::ScanTimer,
};

use mappr_protocols::{self as protocol, ip};
use protocol::ethernet;
use tokio::sync::mpsc::UnboundedSender;

use crate::network::channel::{self, EthernetHandle};

use super::NetworkExplorer;
use async_trait::async_trait;

const MAX_CHANNEL_TIME: Duration = Duration::from_millis(7_500);
const MIN_CHANNEL_TIME: Duration = Duration::from_millis(2_500);
const MAX_SILENCE_MS: Duration = Duration::from_millis(500);
const SEND_INTERVAL_US: Duration = Duration::from_micros(1000);

pub struct LocalScanner {
    hosts_map: HashMap<MacAddr, Host>,
    sender_cfg: SenderConfig,
    eth_handle: EthernetHandle,
    timer: ScanTimer,
    dns_tx: Option<UnboundedSender<IpAddr>>,
    rtt_map: HashMap<IpAddr, Instant>,
}

#[async_trait]
impl NetworkExplorer for LocalScanner {
    async fn discover_hosts(&mut self) -> anyhow::Result<Vec<Host>> {
        let mut packet_iter = protocol::eth_packet_iter(&self.sender_cfg)?;
        let mut sending_finished = false;

        let mut send_interval = tokio::time::interval(SEND_INTERVAL_US);

        let scan_deadline = tokio::time::sleep(MAX_CHANNEL_TIME);
        tokio::pin!(scan_deadline);

        loop {
            if (!self.should_continue() && sending_finished)
                || super::STOP_SIGNAL.load(Ordering::Relaxed)
            {
                break;
            }

            tokio::select! {
                pkt = self.eth_handle.rx.recv() => {
                    match pkt {
                        Some(bytes) => _ = self.process_eth_packet(&bytes),
                        None => break,
                    }
                }

                _ = send_interval.tick(), if !sending_finished => {
                    match packet_iter.next() {
                        Some((packet, ip)) => {
                            self.rtt_map.insert(ip, Instant::now());
                            self.eth_handle.tx.send_to(&packet, None);
                        },
                        None => {
                            sending_finished = true;
                        },
                    }
                }

                _ = &mut scan_deadline => break,
            }
        }

        Ok(self.hosts_map.drain().map(|(_, v)| v).collect())
    }
}

impl LocalScanner {
    pub fn new(
        intf: NetworkInterface,
        collection: IpCollection,
        dns_tx: Option<UnboundedSender<IpAddr>>,
    ) -> anyhow::Result<Self> {
        let eth_handle: EthernetHandle = channel::start_capture(&intf)?;
        let timer = ScanTimer::new(MAX_CHANNEL_TIME, MIN_CHANNEL_TIME, MAX_SILENCE_MS);
        let ips_len: usize = collection.len();

        let mut sender_cfg = SenderConfig::from(&intf);
        sender_cfg.add_packet_type(PacketType::ARP);
        if IS_LAN_SCAN.load(Ordering::Relaxed) {
            sender_cfg.add_packet_type(PacketType::ICMPv6);
        }

        let mut target_ips = std::collections::HashSet::new();

        for single in collection.singles {
            target_ips.insert(single);
        }

        for range in collection.ranges {
            for ip in range.to_iter() {
                target_ips.insert(ip);
            }
        }

        sender_cfg.add_targets(target_ips);

        Ok(Self {
            hosts_map: HashMap::new(),
            sender_cfg,
            eth_handle,
            timer,
            dns_tx,
            rtt_map: HashMap::with_capacity(ips_len),
        })
    }

    fn process_eth_packet(&mut self, bytes: &[u8]) -> anyhow::Result<()> {
        let eth_frame: EthernetPacket = ethernet::get_packet_from_u8(bytes)?;
        if eth_frame.get_source() == self.sender_cfg.local_mac.unwrap() {
            return Ok(());
        }
        let source_addr: IpAddr = protocol::get_ip_addr_from_eth(&eth_frame)?;

        ensure!(
            self.sender_cfg.is_addr_in_subnet(source_addr),
            "{source_addr} is not in range"
        );

        // NOTE: This sucks too as you might tell
        if source_addr.is_ipv6()
            && !IS_LAN_SCAN.load(Ordering::Relaxed)
            && !self.hosts_map.contains_key(&eth_frame.get_source())
        {
            return Ok(());
        }

        let rtt: Option<Duration> = match self.calculate_rtt(&eth_frame) {
            Ok(r) => r,
            Err(e) => {
                error!(verbosity = 2, "Failed to calculate RTT: {e}");
                None
            }
        };

        let source_mac: MacAddr = eth_frame.get_source();

        let mut is_new_host = false;
        let host = self.hosts_map.entry(source_mac).or_insert_with(|| {
            self.timer.mark_seen();
            super::increment_host_count();
            is_new_host = true;
            Host::new(source_addr).with_mac(source_mac)
        });

        if let Some(rtt) = rtt {
            success!(
                verbosity = 2,
                "{source_addr} response in {}ms",
                rtt.as_millis()
            );
            host.add_rtt(rtt);
        }

        let is_new_ip = host.ips.insert(source_addr);

        if source_addr.is_ipv4() && host.primary_ip.is_ipv6() {
            host.primary_ip = source_addr;
        }

        if is_new_host || is_new_ip {
            self.dns_tx.as_ref().map(|tx| tx.send(source_addr));
        }

        Ok(())
    }

    fn calculate_rtt(&mut self, eth_frame: &EthernetPacket) -> anyhow::Result<Option<Duration>> {
        match eth_frame.get_ethertype() {
            EtherTypes::Arp => {
                let arp_packet: ArpPacket = ArpPacket::new(eth_frame.payload())
                    .ok_or_else(|| anyhow!("packet invalid [ARP]"))?;

                let src_addr: IpAddr = IpAddr::V4(arp_packet.get_sender_proto_addr());

                let start_time: Instant = self
                    .rtt_map
                    .remove(&src_addr)
                    .ok_or_else(|| anyhow!("unmapped address [ARP]"))?;

                Ok(Some(start_time.elapsed()))
            }

            EtherTypes::Ipv6 => {
                let dst_addr = match ip::get_ipv6_dst_addr_from_eth(eth_frame) {
                    Ok(addr) => addr,
                    Err(_) => bail!("packet invalid [IPv6]"),
                };

                if dst_addr.is_unicast_link_local() {
                    let dst_addr: IpAddr = IpAddr::V6(dst_addr);
                    let start_time = self
                        .rtt_map
                        .get(&dst_addr)
                        .ok_or_else(|| anyhow!("unmapped link local [IPv6]"))?;

                    return Ok(Some(start_time.elapsed()));
                }

                Ok(None)
            }

            _ => Ok(None),
        }
    }

    fn should_continue(&self) -> bool {
        let not_stopped: bool = !super::STOP_SIGNAL.load(Ordering::Relaxed);
        let time_expired: bool = !self.timer.is_expired();
        let work_remains: bool = self.sender_cfg.len() > self.hosts_map.len();

        not_stopped && time_expired && work_remains
    }
}
