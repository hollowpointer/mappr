use crate::domain::models::host::Host;
use crate::domain::models::target::Target;

#[async_trait::async_trait]
pub trait NetworkScanner: Send + Sync {
    async fn scan(&self, target: Target) -> anyhow::Result<Vec<Box<dyn Host>>>;
}
