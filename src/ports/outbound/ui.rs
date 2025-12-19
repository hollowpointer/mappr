use crate::domain::models::host::Host;

pub trait UserInterface {
    fn display_results(&self, hosts: Vec<Box<dyn Host + Send + Sync>>);
    fn display_error(&self, err: anyhow::Error);
    fn print_header(&self, text: &str);
}
