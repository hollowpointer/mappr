use crate::cmd::Target;
use crate::host::{self, Host, InternalHost};
use crate::net::datalink::interface::NetworkInterfaceExtension;
use crate::net::datalink::{channel, ethernet, interface};
use crate::net::packets::{dns, udp};
use crate::net::{packets, transport};
use crate::net::sender::SenderConfig;
use crate::net::tcp_connect;
use crate::net::ip;
use crate::print::{self, SPINNER};
use anyhow::{self, Context};
use is_root::is_root;
use pnet::datalink::{self, NetworkInterface};
use pnet::packet::Packet;
use pnet::packet::udp::UdpPacket;
use pnet::util::MacAddr;
use rand::random_range;
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

const MAX_CHANNEL_TIME_MS: u64 = 3000;
const MAX_SILENCE_TIME_MS: u64 = 500;

pub async fn discover(target: Target) -> anyhow::Result<()> {
    SPINNER.set_message("Performing discovery...");
    print::print_status("Initializing discovery...");

    let (targets, lan_interface) = get_targets_and_lan_intf(target)?;

    if !is_root() {
        print::print_status("No root privileges. Falling back to non-privileged TCP scan...");
        let mut hosts = host::external_to_box(
            tcp_connect::handshake_range_discovery(targets, tcp_connect::handshake_probe).await?
        );
        return Ok(discovery_ends(&mut hosts)?);
    }

    print::print_status("Root privileges detected. Using advanced techniques...");

    let mut hosts = if let Some(intf) = lan_interface {
        let mut sender_cfg = SenderConfig::from(&intf);
        sender_cfg.add_targets(targets);
        host::internal_to_box(discover_lan(&intf, &sender_cfg)?)
    } else {
        host::external_to_box(
            tcp_connect::handshake_range_discovery(targets, tcp_connect::handshake_probe).await?
        )
    };

    Ok(discovery_ends(&mut hosts)?)
}

fn get_targets_and_lan_intf(target: Target) -> anyhow::Result<(HashSet<IpAddr>, Option<NetworkInterface>)> {
    match target {
        Target::LAN => {
            let intf = interface::get_lan().context("Failed to detect LAN interface for discovery")?;
            let range = intf.get_ipv4_range().context("LAN interface has no valid IPv4 range")?;
            Ok((range.to_iter().collect::<HashSet<_>>(), Some(intf)))
        },
        Target::Host { target_addr } => {
            let intf = if ip::is_private(&target_addr) { interface::get_lan().ok() } else { None };
            Ok((HashSet::from([target_addr]), intf))
        },
        Target::Range { ipv4_range } => {
            let targets: HashSet<IpAddr> = ipv4_range.to_iter().collect();
            let start = IpAddr::V4(ipv4_range.start_addr);
            let end = IpAddr::V4(ipv4_range.end_addr);
            let intf = if ip::is_private(&start) && ip::is_private(&end) {
                interface::get_lan().ok() 
            } else { 
                None 
            };
            Ok((targets, intf))
        },
        Target::VPN => anyhow::bail!("Target::VPN is currently unimplemented!"),
    }
}

pub fn discover_lan(
    intf: &NetworkInterface, 
    sender_cfg: &SenderConfig
) -> anyhow::Result<Vec<InternalHost>> {

    let mut hosts_map: HashMap<MacAddr, InternalHost> = HashMap::new();
    let mut dns_map: HashMap<u16, MacAddr> = HashMap::new(); 

    let (tx_eth, rx_eth) = mpsc::channel();
    let (tx_udp, rx_udp) = mpsc::channel();
    

    let (eth_tx, eth_rx) = channel::open_eth_channel(intf, datalink::channel)?;
    let (mut udp_tx, mut udp_rx) = transport::open_udp_channel()?;

    thread::spawn(move || {
        let mut eth_iter = eth_rx;
        loop {
            if let Ok(frame) = eth_iter.next() {
                if tx_eth.send(frame.to_vec()).is_err() {
                    break;
                }
            }
        }
    });

    thread::spawn(move || {
        let mut udp_iterator = pnet::transport::udp_packet_iter(&mut udp_rx);
        loop {
            if let Ok((udp_packet, _)) = udp_iterator.next() {
                if tx_udp.send(udp_packet.packet().to_vec()).is_err() {
                    break;
                }
            }
        }
    });

    let _ = channel::send_packets(eth_tx, sender_cfg);

    let max_silence = Duration::from_millis(MAX_SILENCE_TIME_MS);
    let hard_deadline = Instant::now() + Duration::from_millis(MAX_CHANNEL_TIME_MS);
    let mut last_seen_packet = Instant::now();

    loop {
        let now = Instant::now();
        if now > hard_deadline {
            break;
        }

        let time_since_last = now.duration_since(last_seen_packet);
        if time_since_last >= max_silence {
            break;
        }

        let remaining_wait = max_silence - time_since_last;
        match rx_eth.recv_timeout(remaining_wait) {
            Ok(bytes) => {
                last_seen_packet = Instant::now();
                if let Ok(eth_frame) = ethernet::get_packet_from_u8(&bytes) {
                    let target_mac = eth_frame.get_source();
            
                    let target_addr = match packets::get_ip_addr_from_eth(&eth_frame) {
                        Ok(ip) => ip,
                        Err(_) => continue, 
                    };

                    if target_addr.is_ipv4() && !sender_cfg.has_addr(&target_addr) {
                        continue;
                    }

                    hosts_map
                        .entry(target_mac)
                        .or_insert_with(|| InternalHost::from(target_mac))
                        .ips
                        .insert(target_addr);

                    if target_addr.is_ipv4() || ip::is_global_unicast(&target_addr) {
                        let id: u16 = ip::derive_u16_id(&target_addr);
                        
                        if !dns_map.contains_key(&id) {
                            dns_map.insert(id, target_mac);
                            
                            if let Ok(ptr_bytes) = dns::create_ptr_packet(&target_addr) {
                                let src_port = random_range(50_000..u16::max_value());
                                if let Ok((dst_addr, dst_port)) = dns::get_dns_server_socket_addr(&target_addr) {
                                     if let Ok(udp_bytes) = udp::create_packet(src_port, dst_port, ptr_bytes) {
                                        if let Some(udp_pkt) = UdpPacket::new(&udp_bytes) {
                                            let _ = udp_tx.send_to(udp_pkt, dst_addr);
                                        }
                                     }
                                }
                            }
                        }
                    }
                }
            },
            Err(mpsc::RecvTimeoutError::Timeout) => {
                break;
            },
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                break;
            }
        }

        while let Ok(bytes) = rx_udp.try_recv() {
            last_seen_packet = Instant::now();
            if let Some(udp_packet) = UdpPacket::new(&bytes) {
                if udp_packet.get_source() == 53 {
                    if let Ok(Some((response_id, name))) = dns::get_hostname(udp_packet.payload()) {
                        if let Some(mac_addr) = dns_map.get(&response_id) {
                            if let Some(host) = hosts_map.get_mut(mac_addr) {
                                host.set_hostname(name);
                            }
                        }
                    }
                }
            }
        }
    }    

    Ok(hosts_map.into_values().collect())
}

fn discovery_ends(hosts: &mut Vec<Box<dyn Host>>) -> anyhow::Result<()>  {
    if hosts.len() == 0 {
        return Ok(no_hosts_found());
    }
    print::header("Network Discovery");
    hosts.sort_by_key(|host| host.get_primary_ip());
    for (idx, host) in hosts.iter().enumerate() {
        host.print_details(idx);
        if idx + 1 != hosts.len() {
            print::println("");
        }
    }
    print::end_of_program();
    SPINNER.finish_and_clear();
    Ok(())
}

fn no_hosts_found() {
    print::header("ZERO HOSTS DETECTED");
    print::no_results();
    print::end_of_program();
    SPINNER.finish_and_clear();
}
