use pnet::{packet::ip::IpNextHeaderProtocols, transport::{self, TransportChannelType, TransportProtocol, TransportReceiver, TransportSender}};


const TRANSPORT_BUFFER_SIZE: usize = 4096;
const CHANNEL_TYPE_UDP: TransportChannelType = TransportChannelType::Layer4(
    TransportProtocol::Ipv4(IpNextHeaderProtocols::Udp)
);

pub fn open_udp_channel() -> anyhow::Result<(TransportSender, TransportReceiver)> {
    let (tx, rx) = transport::transport_channel(TRANSPORT_BUFFER_SIZE, CHANNEL_TYPE_UDP)?;
    Ok((tx, rx))
}