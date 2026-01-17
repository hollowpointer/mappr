use pnet::{
    packet::{Packet, ip::IpNextHeaderProtocols, udp::UdpPacket},
    transport::{
        self, 
        TransportChannelType,
        TransportProtocol,
        TransportReceiver, 
        TransportSender,
    },
};
use std::{net::IpAddr, sync::mpsc};

use mappr_protocols::{dns, udp};

const TRANSPORT_BUFFER_SIZE: usize = 4096;
const CHANNEL_TYPE_UDP: TransportChannelType =
    TransportChannelType::Layer4(TransportProtocol::Ipv4(IpNextHeaderProtocols::Udp));
const CHANNEL_TYPE_TCP: TransportChannelType =
    TransportChannelType::Layer4(TransportProtocol::Ipv4(IpNextHeaderProtocols::Tcp));

#[derive(Debug, Clone, Copy)]
pub enum TransportType {
    TcpLayer4,
    UdpLayer4
}

pub struct TransportHandle {
    pub tx: TransportSender,
    pub rx: mpsc::Receiver<(Vec<u8>, IpAddr)>,
}

macro_rules! spawn_listener {
    ($tx:expr, $rx:expr, $iter_func:path) => {
        std::thread::spawn(move || {
            let mut iterator = $iter_func(&mut $rx);
            loop {
                if let Ok((packet, source_ip)) = iterator.next() {
                    if $tx.send((packet.packet().to_vec(), source_ip)).is_err() {
                        break;
                    }
                }
            }
        })
    };
}

pub fn start_packet_capture(transport_type: TransportType) -> anyhow::Result<TransportHandle> {
    let (tx, mut rx_socket) = open_channel(transport_type)?;
    let (queue_tx, queue_rx) = mpsc::channel();

    match transport_type {
        TransportType::TcpLayer4 => spawn_listener!(queue_tx, rx_socket, pnet::transport::tcp_packet_iter),
        TransportType::UdpLayer4 => spawn_listener!(queue_tx, rx_socket, pnet::transport::udp_packet_iter),
    };

    Ok(TransportHandle { tx, rx: queue_rx })
}

fn open_channel(transport_type: TransportType) -> anyhow::Result<(TransportSender, TransportReceiver)> {
    let channel_type: TransportChannelType = match transport_type {
        TransportType::TcpLayer4 => CHANNEL_TYPE_TCP,
        TransportType::UdpLayer4 => CHANNEL_TYPE_UDP,
    };
    let (tx, rx) = transport::transport_channel(TRANSPORT_BUFFER_SIZE, channel_type)?;
    Ok((tx, rx))
}

pub fn send_dns_query<F>(
    dns_packet_creator_fn: F,
    id: u16,
    target_addr: &IpAddr,
    udp_tx: &mut TransportSender,
) where
    F: Fn(&IpAddr, u16) -> anyhow::Result<Vec<u8>>,
{
    let Ok(bytes) = dns_packet_creator_fn(target_addr, id) else {
        return;
    };

    let Ok((dst_addr, dst_port)) = dns::get_dns_server_socket_addr(&target_addr) else {
        return;
    };

    let src_port = rand::random_range(50_000..u16::max_value());

    let Ok(udp_bytes) = udp::create_packet(src_port, dst_port, bytes) else {
        return;
    };

    let Some(udp_pkt) = UdpPacket::new(&udp_bytes) else {
        return;
    };

    let _ = udp_tx.send_to(udp_pkt, dst_addr);
}

