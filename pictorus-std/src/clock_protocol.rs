pub use std_embedded_time::StandardClock;

pub fn create_clock_protocol() -> StandardClock {
    StandardClock::default()
}
