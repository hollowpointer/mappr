use crate::net::datalink::ethernet;
use crate::net::packets;
use crate::net::sender::SenderConfig;
use crate::print;
use anyhow::{self, Context};
use pnet::datalink;
use pnet::datalink::{Channel, Config, DataLinkReceiver, DataLinkSender, NetworkInterface};
use pnet::packet::ethernet::EthernetPacket;
use std::time::{Duration, Instant};

const PROBE_TIMEOUT_MS: u64 = 2000;

pub struct ChannelRunner<'a> {
    tx: Box<dyn DataLinkSender>,
    rx: Box<dyn DataLinkReceiver>,
    duration_in_ms: Duration,
    sender_cfg: &'a SenderConfig,
    frame_buffer: Vec<Vec<u8>>
}

impl<'a> ChannelRunner<'a> {
    pub fn new(
        intf: &datalink::NetworkInterface,
        sender_cfg: &'a SenderConfig,
    ) -> anyhow::Result<Self> {
        let (tx, rx) = open_eth_channel(intf, &get_config(), datalink::channel)?;
        let duration_in_ms = Duration::from_millis(PROBE_TIMEOUT_MS);
        Ok(Self {
            tx,
            rx,
            duration_in_ms,
            sender_cfg,
            frame_buffer: Vec::new()
        })
    }

    pub fn send_packets(&mut self) -> anyhow::Result<()> {
        let packets: Vec<Vec<u8>> = packets::create_packets(self.sender_cfg)?;
        for packet in packets {
            self.tx.send_to(&packet, None);
        }
        Ok(())
    }

    pub fn receive_eth_frames(&'_ mut self) -> Vec<EthernetPacket<'_>> {
        self.frame_buffer.clear(); 
        let deadline: Instant = Instant::now() + self.duration_in_ms;
        while Instant::now() < deadline {
            if let Ok(frame_bytes) = self.rx.next() {
                self.frame_buffer.push(frame_bytes.to_vec());
            }
        }
        let mut eth_frames: Vec<EthernetPacket> = Vec::new();
        for bytes in &self.frame_buffer {
            if let Ok(frame) = ethernet::get_packet_from_u8(bytes) {
                eth_frames.push(frame);
            }
        }
        
        eth_frames
    }
}

fn open_eth_channel<F>(
    intf: &NetworkInterface,
    cfg: &Config,
    channel_opener: F,
) -> anyhow::Result<(Box<dyn DataLinkSender>, Box<dyn DataLinkReceiver>)>
where F: FnOnce(&NetworkInterface, Config) -> std::io::Result<datalink::Channel>,
{
    let ch: Channel =
        channel_opener(intf, *cfg).with_context(|| format!("opening on {}", intf.name))?;
    match ch {
        Channel::Ethernet(tx, rx) => {
            print::print_status("Connection established successfully");
            Ok((tx, rx))
        }
        _ => anyhow::bail!("non-ethernet channel for {}", intf.name),
    }
}

fn get_config() -> Config {
    Config {
        read_timeout: Some(Duration::from_millis(50)),
        ..Default::default()
    }
}

// ╔════════════════════════════════════════════╗
// ║ ████████╗███████╗███████╗████████╗███████╗ ║
// ║ ╚══██╔══╝██╔════╝██╔════╝╚══██╔══╝██╔════╝ ║
// ║    ██║   █████╗  ███████╗   ██║   ███████╗ ║
// ║    ██║   ██╔══╝  ╚════██║   ██║   ╚════██║ ║
// ║    ██║   ███████╗███████║   ██║   ███████║ ║
// ║    ╚═╝   ╚══════╝╚══════╝   ╚═╝   ╚══════╝ ║
// ╚════════════════════════════════════════════╝

#[cfg(test)]
mod tests {
    use super::*;
    use pnet::datalink::dummy;
    use pnet::datalink::{Config, NetworkInterface};

    #[test]
    fn open_eth_channel_should_succeed_on_ethernet_channel() {
        let dummy_intf: NetworkInterface = dummy::dummy_interface(0);
        let cfg = Config::default();
        let mock_opener_success =
            |i: &NetworkInterface, _cfg: Config| -> std::io::Result<datalink::Channel> {
                let dummy_cfg = pnet::datalink::dummy::Config::default();
                datalink::dummy::channel(i, dummy_cfg)
            };
        let result = open_eth_channel(&dummy_intf, &cfg, mock_opener_success);
        assert!(result.is_ok());
    }

    #[test]
    fn open_eth_channel_should_fail_on_io_error() {
        let dummy_intf: NetworkInterface = dummy::dummy_interface(0);
        let cfg: Config = Config::default();
        let mock_opener_fail =
            |_: &NetworkInterface, _: Config| -> std::io::Result<datalink::Channel> {
                Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "Mock I/O Error",
                ))
            };
        let result = open_eth_channel(&dummy_intf, &cfg, mock_opener_fail);
        assert!(result.is_err());
        if let Err(e) = result {
            let err_msg = e.to_string();
            assert!(err_msg.contains("opening on eth0"));
            let cause: Option<&std::io::Error> = e.downcast_ref::<std::io::Error>();
            assert!(cause.is_some(), "Error cause was not an std::io::Error");
            assert_eq!(cause.unwrap().to_string(), "Mock I/O Error");
            assert_eq!(cause.unwrap().kind(), std::io::ErrorKind::PermissionDenied);
        } else {
            panic!("Test failed: expected Err, got Ok");
        }
    }
}
