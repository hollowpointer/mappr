use std::{
    collections::HashMap,
    net::IpAddr,
    ops::ControlFlow,
    sync::{
        atomic::{AtomicU16, Ordering},
        mpsc,
    },
};

use pnet::{
    packet::{Packet, udp::UdpPacket},
    util::MacAddr,
};

use crate::domain::models::host::Host;

use crate::utils::ip;
use crate::adapters::outbound::network::{
    datalink::{channel::{self, EthernetHandle}, ethernet}, 
    protocol::{self, dns},
    sender::SenderConfig, 
    transport::{self, UdpHandle},
};

use crate::{
    adapters::outbound::terminal::spinner,
    utils::{input::InputHandle, timing::ScanTimer},
};

const DNS_PORT: u16 = 53;
const MDNS_PORT: u16 = 5353;

pub(crate) struct LocalRunner {
    hosts_map: HashMap<MacAddr, Host>,
    dns_map: HashMap<u16, MacAddr>,
    sender_cfg: SenderConfig,
    input_handle: InputHandle,
    eth_handle: EthernetHandle,
    udp_handle: UdpHandle,
    timer: ScanTimer,
    trans_id_counter: AtomicU16,
}

impl LocalRunner {
    pub fn new(
        sender_cfg: SenderConfig,
        input_handle: InputHandle,
        eth_handle: EthernetHandle,
        udp_handle: UdpHandle,
        timer: ScanTimer,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            hosts_map: HashMap::new(),
            dns_map: HashMap::new(),
            sender_cfg,
            input_handle,
            eth_handle,
            udp_handle,
            timer,
            trans_id_counter: AtomicU16::new(0),
        })
    }

    pub fn send_discovery_packets(&mut self) -> anyhow::Result<()> {
        channel::send_packets(&mut self.eth_handle.tx, &self.sender_cfg)?;
        Ok(())
    }

    pub fn start_input_listener(&mut self) {
        self.input_handle.start();
    }

    pub fn process_packets(&mut self) -> ControlFlow<()> {
        if self.timer.is_expired() || self.input_handle.should_interrupt() {
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

        if source_addr.is_ipv4() && !self.sender_cfg.has_addr(&source_addr) {
            return;
        }
        let source_mac = eth_frame.get_source();

        let host = self
            .hosts_map
            .entry(source_mac)
            .or_insert_with(|| Host::new(source_addr).with_mac(source_mac));

        host.ips.insert(source_addr);
        if host.hostname.is_none() {
             self.send_dns_ptr_query(&source_addr, source_mac);
        }
        spinner::report_discovery_progress(self.hosts_map.len());
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
        if !target_addr.is_ipv4() && !ip::is_global_unicast(&target_addr) {
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
            &target_addr,
            &mut self.udp_handle.tx,
        );
    }

    fn get_next_trans_id(&self) -> u16 {
        self.trans_id_counter.fetch_add(1, Ordering::Relaxed)
    }

    pub fn get_hosts(self) -> Vec<Host> {
        self.hosts_map.into_values().collect()
    }
}
