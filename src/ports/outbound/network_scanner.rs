use crate::domain::models::host::Host;
use crate::domain::models::target::Target;

/// Defines the contract for a network scanner adapter.
/// 
/// This trait isolates the application from the underlying network scanning technology
/// (e.g., raw sockets, tokio, or even external tools like nmap).
#[async_trait::async_trait]
pub trait NetworkScanner: Send + Sync {
    /// Scans a specified [`Target`] and returns a list of discovered [`Host`]s.
    ///
    /// # Arguments
    /// * `target` - The target to scan (single IP, Range, or LAN).
    ///
    /// # Returns
    /// A `Result` containing a vector of `Host` objects.
    async fn scan(&self, target: Target) -> anyhow::Result<Vec<Host>>;
}
