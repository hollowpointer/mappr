// use anyhow;
// use pnet::{
//     packet::ip::IpNextHeaderProtocols,
//     transport::{self, TransportChannelType, TransportReceiver, TransportSender},
// };

// use crate::net::range::Ipv4Range;

// struct TransportRunner {
//     tx: TransportSender,
//     rx: TransportReceiver,
// }

// impl TransportRunner {
//     pub fn new() -> anyhow::Result<Self> {
//         let (tx, rx) = open_channel(4096)?;
//         Ok(Self { tx, rx })
//     }

//     // pub fn listen(self) -> anyhow::Result<Vec<InternalHost>> {
//     //     listen_for_hosts(self.rx, self.duration, self.sender_context)
//     // }
// }

// pub fn discover_via_range(ipv4_range: Ipv4Range) -> anyhow::Result<()> {
//     Ok(())
// }

// fn open_channel(buffer_size: usize) -> anyhow::Result<(TransportSender, TransportReceiver)> {
//     let channel_type: TransportChannelType =
//         TransportChannelType::Layer3(IpNextHeaderProtocols::Tcp);
//     Ok(transport::transport_channel(buffer_size, channel_type)?)
// }
