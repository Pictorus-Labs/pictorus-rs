use block_data::BlockData as OldBlockData;
use chrono::{DateTime, Datelike, Local, Timelike};
use pictorus_traits::GeneratorBlock;

/// This block can be used in `std` environments to get the current system time.
/// The time output can be in different formats, such as epoch time, second, minute, hour, day of the month, day of the year, month, or year.
///
/// Optionally, this accepts a sim input, in that case it must be instantiated with `SystemTimeBlock<Sim>`. In this case, the
/// `data` field is assumed to be set externally
pub struct SystemTimeBlock<T: TimeSource = Real> {
    pub data: OldBlockData,
    output: f64,
    start_time: DateTime<Local>,
    _phantom: core::marker::PhantomData<T>,
}

pub trait TimeSource {}
pub struct Sim;
impl TimeSource for Sim {}
pub struct Real;
impl TimeSource for Real {}

impl<T: TimeSource> Default for SystemTimeBlock<T> {
    fn default() -> Self {
        Self {
            data: OldBlockData::from_scalar(0.0),
            output: 0.0,
            start_time: Local::now(),
            _phantom: core::marker::PhantomData,
        }
    }
}

fn get_output_value(time: DateTime<Local>, method: SystemTimeEnum) -> f64 {
    match method {
        SystemTimeEnum::Epoch => time.timestamp() as f64,
        SystemTimeEnum::Second => time.second().into(),
        SystemTimeEnum::Minute => time.minute().into(),
        SystemTimeEnum::Hour => time.hour().into(),
        SystemTimeEnum::DayLunar => time.day().into(),
        SystemTimeEnum::DayOrdinal => time.ordinal().into(),
        SystemTimeEnum::Month => time.month().into(),
        SystemTimeEnum::Year => time.year().into(),
    }
}

impl GeneratorBlock for SystemTimeBlock<Real> {
    type Output = f64;
    type Parameters = Parameters;

    fn generate(
        &mut self,
        parameters: &Self::Parameters,
        context: &dyn pictorus_traits::Context,
    ) -> pictorus_traits::PassBy<Self::Output> {
        // Since simulations can run faster than real-time, we'll use the delta between system start
        // and now, as measured by app_time, for system clock.
        let elpased_time = context.time();
        let time_now = self.start_time + elpased_time;
        self.output = get_output_value(time_now, parameters.method);
        self.data = OldBlockData::from_scalar(self.output);
        self.output
    }
}

impl GeneratorBlock for SystemTimeBlock<Sim> {
    type Output = f64;
    type Parameters = Parameters;

    fn generate(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
    ) -> pictorus_traits::PassBy<Self::Output> {
        // Assume that self.data was set externally and just use that
        self.output = self.data.scalar();
        self.output
    }
}

/// The type of output wanted from the SystemTimeBlock.
/// Has no affect when used with a sim input
#[derive(strum::EnumString, Clone, Copy, Debug)]
pub enum SystemTimeEnum {
    Epoch,
    Second,
    Minute,
    Hour,
    DayLunar,
    DayOrdinal,
    Month,
    Year,
}

/// Parameters for the SystemTimeBlock
pub struct Parameters {
    pub method: SystemTimeEnum,
}

impl Parameters {
    pub fn new(method: &str) -> Parameters {
        Parameters {
            method: method.parse().unwrap(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::testing::StubContext;

    #[test]
    fn test_get_output_value() {
        let time = Local::now();
        let epoch = get_output_value(time, SystemTimeEnum::Epoch);
        let second = get_output_value(time, SystemTimeEnum::Second);
        let minute = get_output_value(time, SystemTimeEnum::Minute);
        let hour = get_output_value(time, SystemTimeEnum::Hour);
        let day_lunar = get_output_value(time, SystemTimeEnum::DayLunar);
        let day_ordinal = get_output_value(time, SystemTimeEnum::DayOrdinal);
        let month = get_output_value(time, SystemTimeEnum::Month);
        let year = get_output_value(time, SystemTimeEnum::Year);

        assert_eq!(epoch, time.timestamp() as f64);
        assert_eq!(second, time.second() as f64);
        assert_eq!(minute, time.minute() as f64);
        assert_eq!(hour, time.hour() as f64);
        assert_eq!(day_lunar, time.day() as f64);
        assert_eq!(day_ordinal, time.ordinal() as f64);
        assert_eq!(month, time.month() as f64);
        assert_eq!(year, time.year() as f64);
    }

    #[test]
    fn test_system_time_block() {
        let mut block: SystemTimeBlock = Default::default();
        let start_time = block.start_time;
        assert!(Local::now() >= start_time);
        assert!(Local::now() <= start_time + chrono::Duration::milliseconds(100));

        let params = Parameters::new("Epoch");
        let context = StubContext::new(
            Duration::from_secs(42),
            Some(Duration::from_millis(100)),
            Duration::from_millis(100),
        );
        let output = block.generate(&params, &context);
        assert_eq!(output, start_time.timestamp() as f64 + 42.0);
        assert_eq!(block.data.scalar(), start_time.timestamp() as f64 + 42.0);
    }

    #[test]
    fn test_system_time_block_sim() {
        let mut block: SystemTimeBlock<Sim> = Default::default();
        block.data.set_scalar(1337.0);
        let params = Parameters::new("Epoch");
        let context = StubContext::new(
            Duration::from_secs(42),
            Some(Duration::from_millis(100)),
            Duration::from_millis(100),
        );
        let output = block.generate(&params, &context);
        assert_eq!(output, 1337.0);
        assert_eq!(block.data.scalar(), 1337.0);
    }
}
