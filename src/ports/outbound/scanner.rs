use anyhow::Result;
use tokio::sync::mpsc;

use crate::domain::models::host::Host;
use crate::domain::models::target::Target;

pub trait NetworkScanner {
    fn discover(&self, target: Target) -> mpsc::Receiver<Result<Box<dyn Host + Send + Sync>>>;
}
