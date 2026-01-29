pub struct Config {
    /// Keep logs and colors but hide the ASCII art
    pub no_banner: bool,

    /// Disables scanners from sending DNS packets.
    ///
    /// Does not stop the scanners from accepting DNS packets.
    pub no_dns: bool,

    /// Redact sensitive info (IPv6 suffixes, MAC addresses etc.)
    pub redact: bool,

    /// Reduce UI visual density (1: reduce styling, 2: raw IPs)
    pub quiet: u8,

    /// Disable user input listening (e.g. for non-interactive tests)
    pub disable_input: bool,
}
