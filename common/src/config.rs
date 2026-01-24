pub struct Config {
    /// Disables scanners from sending DNS packets.
    /// 
    /// Does not stop the scanners from accepting DNS packets.
    pub no_dns: bool,
    pub redact: bool
}