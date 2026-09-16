pub mod config;
pub mod db;
pub mod errors;
pub mod log;
pub mod payload_buffer;
pub mod views;

pub trait View: Default {
    fn handle_events(&mut self, events: &[u8]) -> errors::Result<()>;
    fn take_events(&mut self) -> Vec<u8>;
}
