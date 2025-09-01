use std::net::Ipv4Addr;
use std::thread;
use std::time::{Duration, Instant};
use anyhow::{anyhow, bail, Context, Result};
use mac_oui::Oui;
use pnet::datalink;
use pnet::datalink::{Channel, Config, DataLinkReceiver, DataLinkSender, NetworkInterface};
use crate::cmd::discover::Host;
use crate::net::packets;
use crate::net::packets::{CraftedPacket, PacketType};
use crate::net::range::ip_iter;
use crate::print;

fn send(intf: &NetworkInterface,
        ip: Ipv4Addr,
        packet_type: PacketType,
        tx: &mut Box<dyn DataLinkSender>
) -> Result<()> {
    let pkt = CraftedPacket::new(packet_type, &intf, ip)?;
    if let Some(Err(e)) = tx.send_to(pkt.bytes(), Some(intf.clone())) {
        eprintln!("send {ip} failed: {e}");
    }
    Ok(())
}

fn send_sweep(start: Ipv4Addr,
              end: Ipv4Addr,
              intf: &NetworkInterface,
              packet_type: PacketType,
              tx: &mut Box<dyn DataLinkSender>,
) {
    let len: u64 = u64::from(u32::from(end) - u32::from(start) + 1);
    let progress_bar = print::create_progressbar(len, format!("{:?}", packet_type));
    for ip in ip_iter((start, end)) {
        send(&intf, ip, packet_type, tx).expect("Failed to perform ARP sweep");
        progress_bar.inc(1);
        thread::sleep(Duration::from_millis(5));
    }
}

pub fn discover_hosts_on_eth_channel(
    start: Ipv4Addr,
    end: Ipv4Addr,
    intf: NetworkInterface,
    mut channel_cfg: Config,
    duration_in_ms: Duration,
) -> Result<Vec<Host>> {
    if channel_cfg.read_timeout.is_none() {
        channel_cfg.read_timeout = Some(Duration::from_millis(50));
    }
    let oui_db = Oui::default().map_err(|e| { anyhow!("loading OUI database: {}", e) })?;
    let (mut tx, mut rx) = open_ethernet_channel(&intf, &channel_cfg)?;
    if u32::from(start) > u32::from(end) { bail!("end IP ({end}) must be >= start IP ({start})"); }
    print::print_status("Connection established. Beginning sweep...");
    send_sweep(start, end, &intf, PacketType::ARP, &mut tx);
    let mut hosts: Vec<Host> = Vec::new();
    let deadline = Instant::now() + duration_in_ms;
    while deadline > Instant::now() {
        match rx.next() {
            Ok(frame) => {
                if let Some(host) = packets::handle_frame(&frame, &oui_db).ok() {
                    hosts.extend(host);
                }
            },
            Err(_) => { }
        }
    }
    Ok(hosts)
}

fn open_ethernet_channel(intf: &NetworkInterface, cfg: &Config)
                             -> Result<(Box<dyn DataLinkSender>, Box<dyn DataLinkReceiver>)> {
    let ch = datalink::channel(intf, *cfg)
        .with_context(|| format!("opening on {}", intf.name))?;
    match ch {
        Channel::Ethernet(tx, rx) => Ok((tx, rx)),
        _ => bail!("non-ethernet channel for {}", intf.name),
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
    use std::io;
    use std::net::Ipv4Addr;
    use std::sync::{Arc, Mutex};
    use pnet::datalink::{DataLinkSender, MacAddr, NetworkInterface};
    use pnet::ipnetwork::{IpNetwork, Ipv4Network};
    use crate::net::channel::send_sweep;
    use crate::net::packets::PacketType;

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

    #[test]
    fn send_sweep_calls_send_to_for_each_ip_even_if_one_fails() {
        // 3 IPs in range
        let start = Ipv4Addr::new(192, 168, 1, 1);
        let end   = Ipv4Addr::new(192, 168, 1, 3);

        let intf = dummy_iface();
        let (mut tx, sent_counter) = FakeSender::new(true);

        // should not panic and should attempt all three sends
        send_sweep(start, end, &intf, PacketType::ARP, &mut tx);

        let sent = *sent_counter.lock().unwrap();
        assert_eq!(sent, 3, "expected one send per IP in range");
    }
}