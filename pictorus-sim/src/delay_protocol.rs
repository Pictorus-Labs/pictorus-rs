use embedded_hal::delay::DelayNs;

pub struct SimDelayProtocol {}

impl DelayNs for SimDelayProtocol {
    fn delay_ns(&mut self, _: u32) {}
}

pub fn create_delay_protocol() -> SimDelayProtocol {
    SimDelayProtocol {}
}
