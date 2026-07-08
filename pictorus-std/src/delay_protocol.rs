use embedded_hal::delay::DelayNs;
use embedded_hal_02::blocking::delay::{DelayMs, DelayUs};
use std::thread;
use std::time::Duration;

#[derive(Default)]
pub struct StdDelayProtocol {}

impl StdDelayProtocol {
    pub fn new() -> Self {
        Self {}
    }
}

impl<UXX: Into<u64>> DelayMs<UXX> for StdDelayProtocol {
    fn delay_ms(&mut self, ms: UXX) {
        thread::sleep(Duration::from_millis(ms.into()));
    }
}

impl<UXX: Into<u64>> DelayUs<UXX> for StdDelayProtocol {
    fn delay_us(&mut self, us: UXX) {
        thread::sleep(Duration::from_micros(us.into()));
    }
}

impl DelayNs for StdDelayProtocol {
    fn delay_ns(&mut self, ns: u32) {
        thread::sleep(Duration::from_nanos(ns.into()));
    }
}

pub fn create_delay_protocol() -> StdDelayProtocol {
    StdDelayProtocol::new()
}
