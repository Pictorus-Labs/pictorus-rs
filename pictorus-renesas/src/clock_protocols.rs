// RA4M2 HAL implements a Clock trait and RenesasClock struct
#[cfg(feature="ra4m2")]
pub use ra4m2_hal::time_driver::RenesasClock;


// use embedded_time::{Clock, Instant, rate::Fraction};



// #[derive(Default)]
// pub struct RenesasClock {}

// impl Clock for RenesasClock {
//     type T = u64;

//     // TODO do some error checking. This technically will fail with clocks above 4 GHz
//     const SCALING_FACTOR: Fraction = Fraction::new(1, embassy_time::TICK_HZ as u32);

//     fn try_now(&self) -> Result<Instant<Self>, embedded_time::clock::Error> {
//         Ok(Instant::new(embassy_time::Instant::now().as_ticks()))
//     }
// }
