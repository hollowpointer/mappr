//! A **local area network (LAN)** scanner.
//!
//! Primarily used for discovering and scanning hosts on the same physical network,
//! using protocols like ARP, NDP, and ICMP for discovery and TCP/UDP for port scanning.
//!
//! This scanner requires **root privileges** to construct and intercept raw
//! Layer 2 packets via the operating system's network sockets.

use std::{collections::HashMap, net::IpAddr, sync::atomic::Ordering, time::Duration};

use anyhow::ensure;
use pnet::{datalink::NetworkInterface, util::MacAddr};

use mappr_common::{
    error,
    network::{host::Host, range::IpCollection, target::IS_LAN_SCAN},
    sender::{PacketType, SenderConfig},
    utils::timing::ScanTimer,
};

use mappr_protocols as protocol;
use protocol::ethernet;
use tokio::sync::mpsc::UnboundedSender;

use crate::network::channel::{self, EthernetHandle};

use super::NetworkExplorer;
use async_trait::async_trait;

const MAX_CHANNEL_TIME: Duration = Duration::from_millis(7_500);
const MIN_CHANNEL_TIME: Duration = Duration::from_millis(2_500);
const MAX_SILENCE: Duration = Duration::from_millis(500);

pub struct LocalScanner {
    hosts_map: HashMap<MacAddr, Host>,
    sender_cfg: SenderConfig,
    eth_handle: EthernetHandle,
    timer: ScanTimer,
    dns_tx: Option<UnboundedSender<IpAddr>>,
}

#[async_trait]
impl NetworkExplorer for LocalScanner {
    async fn discover_hosts(&mut self) -> anyhow::Result<Vec<Host>> {
        if let Err(e) = self.send_discovery_packets() {
            error!("Failed to send discovery packets: {e}");
        }

        let scan_deadline = tokio::time::sleep(MAX_CHANNEL_TIME);
        tokio::pin!(scan_deadline);

        loop {
            if !self.should_continue() || super::STOP_SIGNAL.load(Ordering::Relaxed) {
                break;
            }

            let silence_timeout = tokio::time::sleep(MAX_SILENCE);

            tokio::select! {
                pkt = self.eth_handle.rx.recv() => {
                    match pkt {
                        Some(bytes) => _ = self.process_eth_packet(&bytes),
                        None => break,
                    }
                }

                _ = &mut scan_deadline => {
                    break;
                }

                _ = silence_timeout => {
                    if self.timer.should_break_on_timeout() {
                        break;
                    }
                }
            }
        }

        let hosts: Vec<Host> = self.hosts_map.drain().map(|(_, v)| v).collect();
        Ok(hosts)
    }
}

impl LocalScanner {
    pub fn new(
        intf: NetworkInterface,
        collection: IpCollection,
        dns_tx: Option<UnboundedSender<IpAddr>>,
    ) -> anyhow::Result<Self> {
        let eth_handle: EthernetHandle = channel::start_capture(&intf)?;
        let timer = ScanTimer::new(MAX_CHANNEL_TIME, MIN_CHANNEL_TIME, MAX_SILENCE);

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
        })
    }

    fn send_discovery_packets(&mut self) -> anyhow::Result<()> {
        let packets: Vec<Vec<u8>> = protocol::create_ethernet_packets(&self.sender_cfg)?;
        for packet in packets {
            self.eth_handle.tx.send_to(&packet, None);
        }
        Ok(())
    }

    fn process_eth_packet(&mut self, bytes: &[u8]) -> anyhow::Result<()> {
        let eth_frame = ethernet::get_packet_from_u8(bytes)?;
        let source_addr = protocol::get_ip_addr_from_eth(&eth_frame)?;
        ensure!(
            self.sender_cfg.is_addr_in_subnet(source_addr),
            "{source_addr} is not in range"
        );

        // Needs rework
        if source_addr.is_ipv6()
            && !IS_LAN_SCAN.load(Ordering::Relaxed)
            && !self.hosts_map.contains_key(&eth_frame.get_source())
        {
            return Ok(());
        }

        let source_mac = eth_frame.get_source();

        let mut is_new_host = false;
        let host = self.hosts_map.entry(source_mac).or_insert_with(|| {
            self.timer.mark_seen();
            super::increment_host_count();
            is_new_host = true;
            Host::new(source_addr).with_mac(source_mac)
        });

        let is_new_ip = host.ips.insert(source_addr);

        if is_new_host || is_new_ip {
            self.dns_tx.as_ref().map(|tx| tx.send(source_addr));
        }

        Ok(())
    }

    fn should_continue(&self) -> bool {
        let not_stopped: bool = !super::STOP_SIGNAL.load(Ordering::Relaxed);
        let time_expired: bool = !self.timer.is_expired();
        let work_remains: bool = self.sender_cfg.len() > self.hosts_map.len();

        not_stopped && time_expired && work_remains
    }
}

