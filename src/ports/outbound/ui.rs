use crate::domain::models::host::Host;

pub trait UserInterface {
    fn display_results(&self, hosts: Vec<Box<dyn Host + Send + Sync>>);
    fn display_error(&self, err: anyhow::Error);
    // Add other methods as needed based on original bin/mappr.rs
    fn print_header(&self, text: &str);
}
