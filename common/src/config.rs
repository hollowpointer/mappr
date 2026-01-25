pub struct Config {
    /// Disables scanners from sending DNS packets.
    /// 
    /// Does not stop the scanners from accepting DNS packets.
    pub no_dns: bool,

    /// Redact sensitive info (IPv6 suffixes, MAC addresses etc.)
    pub redact: bool,

    /// Reduce UI visual density (1: reduce styling, 2: raw IPs)
    pub quiet: u8,
}