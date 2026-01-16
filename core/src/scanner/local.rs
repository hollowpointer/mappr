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
    ops::ControlFlow,
    sync::atomic::{AtomicU16, Ordering},
    sync::mpsc,
    time::Duration,
};

use pnet::{
    datalink::NetworkInterface,
    packet::{Packet, udp::UdpPacket},
    util::MacAddr,
};

use mappr_common::{
    config::{PacketType, SenderConfig},
    network::{host::Host, range::IpCollection, target::IS_LAN_SCAN},
    utils::timing::ScanTimer,
};

use mappr_protocols as protocol;
use protocol::{dns, ethernet};
use tracing::{error, info};

use crate::network::{
    channel::{self, EthernetHandle},
    transport::{self, TransportHandle, TransportType},
};

use super::NetworkExplorer;
use async_trait::async_trait;

const DNS_PORT: u16 = 53;
const MDNS_PORT: u16 = 5353;
const MAX_CHANNEL_TIME: Duration = Duration::from_millis(7_500);
const MIN_CHANNEL_TIME: Duration = Duration::from_millis(2_500);
const MAX_SILENCE: Duration = Duration::from_millis(500);

pub struct LocalScanner {
    hosts_map: HashMap<MacAddr, Host>,
    dns_map: HashMap<u16, MacAddr>,
    sender_cfg: SenderConfig,
    eth_handle: EthernetHandle,
    udp_handle: TransportHandle,
    timer: ScanTimer,
    trans_id_counter: AtomicU16,
}

impl LocalScanner {
    pub fn new(
        intf: NetworkInterface,
        collection: IpCollection,
    ) -> anyhow::Result<Self> {
        let eth_handle: EthernetHandle = channel::start_capture(&intf)?;
        let udp_handle: TransportHandle = transport::start_packet_capture(TransportType::UdpLayer4)?;
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
            dns_map: HashMap::new(),
            sender_cfg,
            eth_handle,
            udp_handle,
            timer,
            trans_id_counter: AtomicU16::new(0),
        })
    }

    fn send_discovery_packets(&mut self) -> anyhow::Result<()> {
        match channel::send_packets(&mut self.eth_handle.tx, &self.sender_cfg) {
            Ok(_) => info!("Discovery packets have been sent successfully"),
            Err(e) => error!("Failed to send discovery packets: {e}"),
        }
        Ok(())
    }

    fn process_packets(&mut self) -> ControlFlow<()> {
        if self.timer.is_expired() || super::STOP_SIGNAL.load(Ordering::Relaxed) {
            return ControlFlow::Break(());
        }

        let wait = self.timer.next_wait();

        match self.eth_handle.rx.recv_timeout(wait) {
            Ok(bytes) => {
                self.timer.mark_seen();
                self.process_eth_packet(&bytes);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if self.timer.should_break_on_timeout() {
                    return ControlFlow::Break(());
                }
                return ControlFlow::Continue(());
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return ControlFlow::Break(());
            }
        }

        self.process_udp_packets();
        ControlFlow::Continue(())
    }

    fn process_eth_packet(&mut self, bytes: &[u8]) {
        let Ok(eth_frame) = ethernet::get_packet_from_u8(bytes) else {
            return;
        };

        let Ok(source_addr) = protocol::get_ip_addr_from_eth(&eth_frame) else {
            return;
        };

        if !self.sender_cfg.is_addr_in_subnet(source_addr) {
            return;
        }

        if source_addr.is_ipv4() && !self.sender_cfg.has_addr(&source_addr) {
            return;
        } 
        
        if source_addr.is_ipv6() {
            if !IS_LAN_SCAN.load(Ordering::Relaxed) {
                if !self.hosts_map.contains_key(&eth_frame.get_source()) {
                    return
                }
            }
        }

        let source_mac = eth_frame.get_source();

        let host = self.hosts_map.entry(source_mac).or_insert_with(|| {
            super::increment_host_count();
            Host::new(source_addr).with_mac(source_mac)
        });

        host.ips.insert(source_addr);
        if host.hostname.is_none() {
            self.send_dns_ptr_query(&source_addr, source_mac);
        }
    }

    fn process_udp_packets(&mut self) {
        while let Ok(bytes) = self.udp_handle.rx.try_recv() {
            self.timer.mark_seen();
            let Some(udp_packet) = UdpPacket::new(&bytes) else {
                continue;
            };
            match udp_packet.get_source() {
                DNS_PORT => self.handle_dns_response(udp_packet),
                MDNS_PORT => { /* Implement mDNS next */ }
                _ => continue,
            }
        }
    }

    fn handle_dns_response(&mut self, packet: UdpPacket) {
        let Ok(Some((response_id, name))) = dns::get_hostname(packet.payload()) else {
            return;
        };
        let Some(mac_addr) = self.dns_map.get(&response_id) else {
            return;
        };
        if let Some(host) = self.hosts_map.get_mut(mac_addr) {
            host.hostname = Some(name);
        }
    }

    fn send_dns_ptr_query(&mut self, target_addr: &IpAddr, target_mac: MacAddr) {
        if !target_addr.is_ipv4() && !mappr_common::utils::ip::is_global_unicast(target_addr) {
            return;
        }
        let id = self.get_next_trans_id();
        if self.dns_map.contains_key(&id) {
            return;
        }
        self.dns_map.insert(id, target_mac);
        transport::send_dns_query(
            dns::create_ptr_packet,
            id,
            target_addr,
            &mut self.udp_handle.tx,
        );
    }

    fn get_next_trans_id(&self) -> u16 {
        self.trans_id_counter.fetch_add(1, Ordering::Relaxed)
    }
}

#[async_trait]
impl NetworkExplorer for LocalScanner {
    fn discover_hosts(&mut self) -> anyhow::Result<Vec<Host>> {
        self.send_discovery_packets()?;
        loop {
            if let ControlFlow::Break(_) = self.process_packets() {
                break;
            }
        }

        Ok(self.hosts_map.drain().map(|(_, v)| v).collect())
    }
}
