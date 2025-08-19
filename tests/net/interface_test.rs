use pnet::datalink::{MacAddr, NetworkInterface};
use mappr::cmd::Target;
use mappr::net::interface;
use super::util::{ni, v4, v6};

/*************************************************************
                       Tests for LAN
**************************************************************/

#[test]
fn lan_selects_enp9s0() {
    let interfaces: Vec<NetworkInterface> = iface_all();
    assert_eq!(enp9s0(), interface::select(Target::LAN, &interfaces).unwrap());
}

#[test]
fn lan_selects_nothing_when_no_lan() {
    let interfaces: Vec<NetworkInterface> = vec![lo(), veth1234(), tun0(), ipv6leakintrf0()];
    let selected = interface::select(Target::LAN, &interfaces);
    assert!(selected.is_none(), "Expected no interface, received: {selected:?}");
}

#[test]
fn lan_selects_wlan0() {
    let interfaces: Vec<NetworkInterface> = vec![ipv6leakintrf0(), lo(), veth1234(), wlan0()];
    assert_eq!(wlan0(), interface::select(Target::LAN, &interfaces).unwrap());
}

#[test]
fn lan_selects_eth1() {
    let interfaces: Vec<NetworkInterface> = vec![lo(), ipv6leakintrf0(), veth1234(), eth1()];
    assert_eq!(eth1(), interface::select(Target::LAN, &interfaces).unwrap());
}

/*************************************************************
                  Mock interfaces for testing
**************************************************************/

fn iface_all() -> Vec<NetworkInterface> {
    vec![lo(),
         enp9s0(),
         tun0(),
         ipv6leakintrf0(),
         wlan0(),
         eth1(),
         docker0(),
         veth1234(),
         br0()
    ]
}

fn lo() -> NetworkInterface {
    ni(
        "lo",
        1,
        Some(MacAddr::new(0, 0, 0, 0, 0, 0)),
        &[v4(127, 0, 0, 1, 8), v6("::1", 128)],
        65609,
    )
}

fn enp9s0() -> NetworkInterface {
    ni(
        "enp9s0",
        2,
        Some(MacAddr::new(0xa8, 0xa1, 0x59, 0x13, 0x41, 0x46)),
        &[
            v4(192, 168, 0, 32, 24),
            v6("2a02:908:8c1:b880::b054", 128),
            v6("2a02:908:8c1:b880:97f7:c408:8dff:b5bf", 64),
            v6("fe80::b3dd:5c39:7c29:48b6", 64),
        ],
        69699,
    )
}

fn tun0() -> NetworkInterface {
    ni(
        "tun0",
        5,
        None,
        &[v4(10, 96, 0, 57, 16), v6("fe80::c137:8964:5a63:efde", 64)],
        69841,
    )
}

fn ipv6leakintrf0() -> NetworkInterface {
    ni(
        "ipv6leakintrf0",
        6,
        Some(MacAddr::new(0xd2, 0x25, 0xd4, 0x9f, 0x18, 0xfd)),
        &[v6("fdeb:446c:912d:8da::", 64), v6("fe80::7f87:ff4a:9ad8:d2f0", 64)],
        65731,
    )
}

fn wlan0() -> NetworkInterface {
    ni(
        "wlan0",
        3,
        Some(MacAddr::new(0x34, 0xcf, 0xf6, 0x9a, 0x11, 0x22)),
        &[
            v4(192, 168, 1, 42, 24),
            v6("fe80::36cf:f6ff:fe9a:1122", 64),
        ],
        69699,
    )
}

fn eth1() -> NetworkInterface {
    ni(
        "eth1",
        4,
        Some(MacAddr::new(0x52, 0x54, 0x00, 0x12, 0x34, 0x56)),
        &[v4(10, 0, 0, 15, 24)],
        69699,
    )
}

fn docker0() -> NetworkInterface {
    ni(
        "docker0",
        7,
        Some(MacAddr::new(0x02, 0x42, 0xac, 0x11, 0x00, 0x01)),
        &[v4(172, 17, 0, 1, 16)],
        69699,
    )
}

fn veth1234() -> NetworkInterface {
    ni(
        "veth1234",
        8,
        Some(MacAddr::new(0x1a, 0x2b, 0x3c, 0x4d, 0x5e, 0x6f)),
        &[v6("fe80::1a2b:3cff:fe4d:5e6f", 64)],
        69699,
    )
}

fn br0() -> NetworkInterface {
    ni(
        "br0",
        9,
        Some(MacAddr::new(0xde, 0xad, 0xbe, 0xef, 0x00, 0x01)),
        &[
            v4(192, 168, 100, 1, 24),
            v6("fd00:dead:beef::1", 64),
        ],
        69699,
    )
}