use std::net::{Ipv4Addr, Ipv6Addr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use anyhow;
use anyhow::{bail, Context};
use pnet::datalink;
use pnet::datalink::{Channel, Config, DataLinkReceiver, DataLinkSender, NetworkInterface};
use pnet::util::MacAddr;
use crate::cmd::Target;
use crate::net::packets::icmp;
use crate::host::Host;
use crate::net::{packets, range};
use crate::net::datalink::{interface, arp};
use crate::net::range::Ipv4Range;
use crate::print;

pub enum ProbeType {
    Default
}

pub struct SenderContext {
    pub ipv4range: Arc<Ipv4Range>,
    pub src_addr_v4: Ipv4Addr,
    pub src_addr_v6: Ipv6Addr,
    pub mac_addr: MacAddr,
    pub tx: Box<dyn DataLinkSender>
}

impl SenderContext {
    fn new(ipv4range: Arc<Ipv4Range>,
           src_addr_v4: Ipv4Addr,
           src_addr_v6: Ipv6Addr,
           intf: Arc<NetworkInterface>,
           tx: Box<dyn DataLinkSender>)
     -> Self {
        Self {
            ipv4range,
            src_addr_v4,
            src_addr_v6,
            mac_addr: intf.mac.unwrap(),
            tx,
        }
    }
}

pub fn discover_on_eth_channel(ipv4range: Arc<Ipv4Range>,
                               intf: Arc<NetworkInterface>,
                               channel_cfg: Config,
                               probe_type: ProbeType,
                               duration_in_ms: Duration
) -> anyhow::Result<Vec<Host>> {
    let (tx, rx) = open_eth_channel(&intf, &channel_cfg)?;
    let _ = range::ip_range(Target::LAN, &intf); // this shit is to suppress warnings
    let src_addr_v4: Ipv4Addr = interface::get_ipv4(&intf)?;
    let src_addr_v6: Ipv6Addr = interface::get_ipv6(&intf)?;
    let mut send_context: SenderContext = SenderContext::new(ipv4range, src_addr_v4, src_addr_v6, intf, tx);
    match probe_type {
        ProbeType::Default => {
            arp::send_packets(&mut send_context)?;
            icmp::send_echo_request_v6(&mut send_context)?;
        },
    }
    Ok(listen_for_hosts(rx, duration_in_ms))
}

fn open_eth_channel(intf: &NetworkInterface, cfg: &Config)
    -> anyhow::Result<(Box<dyn DataLinkSender>, Box<dyn DataLinkReceiver>)> {
    let ch = datalink::channel(intf, *cfg).with_context(|| format!("opening on {}", intf.name))?;
    match ch {
        Channel::Ethernet(tx, rx) => {
            print::print_status("Connection established. Beginning sweep...");
            Ok((tx, rx))
        },
        _ => bail!("non-ethernet channel for {}", intf.name),
    }
}

fn listen_for_hosts(mut rx: Box<dyn DataLinkReceiver>, duration_in_ms: Duration) -> Vec<Host> {
    let mut hosts: Vec<Host> = Vec::new();
    let deadline = Instant::now() + duration_in_ms;
    while deadline > Instant::now() {
        match rx.next() {
            Ok(frame) => {
                if let Some(host) = packets::handle_frame(&frame).ok()
                { hosts.extend(host); }
            },
            Err(_) => { }
        }
    }
    hosts
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
    use std::io;
    use std::net::Ipv4Addr;
    use std::sync::{Arc, Mutex};
    use pnet::datalink::{DataLinkSender, MacAddr, NetworkInterface};
    use pnet::ipnetwork::{IpNetwork, Ipv4Network};

    // ---- Fake sender to spy on send_sweep ----
    struct FakeSender {
        sent: Arc<Mutex<usize>>,
        fail_first: bool,
        calls: usize,
    }

    impl FakeSender {
        fn new(fail_first: bool) -> (Box<dyn DataLinkSender>, Arc<Mutex<usize>>) {
            let sent = Arc::new(Mutex::new(0usize));
            let s = FakeSender { sent: sent.clone(), fail_first, calls: 0 };
            (Box::new(s), sent)
        }
    }

    impl DataLinkSender for FakeSender {
        fn build_and_send(
            &mut self,
            _num_packets: usize,
            _packet_size: usize,
            _func: &mut dyn for<'a> FnMut(&'a mut [u8]),
        ) -> Option<io::Result<()>> {
            // not used by our code-path
            Some(Ok(()))
        }

        fn send_to(
            &mut self,
            _packet: &[u8],
            _dst: Option<NetworkInterface>,
        ) -> Option<io::Result<()>> {
            self.calls += 1;
            *self.sent.lock().unwrap() += 1;
            if self.fail_first && self.calls == 1 {
                return Some(Err(io::Error::new(io::ErrorKind::Other, "boom")));
            }
            Some(Ok(()))
        }
    }

    fn dummy_iface() -> NetworkInterface {
        NetworkInterface {
            name: "test0".into(),
            description: "".to_string(),
            index: 1,
            mac: Some(MacAddr::new(0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff)),
            ips: vec![IpNetwork::V4(
                Ipv4Network::new(Ipv4Addr::new(192, 168, 1, 100), 24).unwrap()
            )],
            flags: 0,
        }
    }
}