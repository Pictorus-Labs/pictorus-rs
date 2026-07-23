use chrono::{DateTime, Datelike, Local, Timelike};
use num_traits::AsPrimitive;
use pictorus_traits::{GeneratorBlock, PassBy};

use crate::traits::Float;

/// This block can be used in `std` environments to get the current system time.
/// The time output can be in different formats, such as epoch time, second, minute, hour, day of the month, day of the year, month, or year.
pub struct SystemTimeBlock<O: Float = f64> {
    output: O,
    start_time: DateTime<Local>,
}

impl<O: Float> Default for SystemTimeBlock<O> {
    fn default() -> Self {
        Self {
            output: O::zero(),
            start_time: Local::now(),
        }
    }
}

fn get_output_value<O: Float>(time: DateTime<Local>, method: SystemTimeEnum) -> O
where
    i64: AsPrimitive<O>,
    i32: AsPrimitive<O>,
    u32: AsPrimitive<O>,
{
    // As casts are technically lossy but should be ok for the ranges of values we expect here
    match method {
        SystemTimeEnum::Epoch => time.timestamp().as_(),
        SystemTimeEnum::Second => time.second().as_(),
        SystemTimeEnum::Minute => time.minute().as_(),
        SystemTimeEnum::Hour => time.hour().as_(),
        SystemTimeEnum::DayLunar => time.day().as_(),
        SystemTimeEnum::DayOrdinal => time.ordinal().as_(),
        SystemTimeEnum::Month => time.month().as_(),
        SystemTimeEnum::Year => time.year().as_(),
    }
}

impl<O: Float> GeneratorBlock for SystemTimeBlock<O>
where
    i64: AsPrimitive<O>,
    i32: AsPrimitive<O>,
    u32: AsPrimitive<O>,
{
    type Output = O;
    type Parameters = Parameters;

    fn generate(
        &mut self,
        parameters: &Self::Parameters,
        context: &dyn pictorus_traits::Context,
    ) -> pictorus_traits::PassBy<'_, Self::Output> {
        // Since simulations can run faster than real-time, we'll use the delta between system start
        // and now, as measured by app_time, for system clock.
        let elapsed_time = context.time();
        let time_now = self.start_time + elapsed_time;
        self.output = get_output_value(time_now, parameters.method);
        self.output
    }

    fn buffer(&self) -> PassBy<'_, Self::Output> {
        self.output
    }
}

/// The type of output wanted from the SystemTimeBlock.
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
        let epoch: f32 = get_output_value(time, SystemTimeEnum::Epoch);
        assert!(epoch.is_finite());
        let second: f32 = get_output_value(time, SystemTimeEnum::Second);
        assert!(second.is_finite());
        let minute: f32 = get_output_value(time, SystemTimeEnum::Minute);
        assert!(minute.is_finite());
        let hour: f32 = get_output_value(time, SystemTimeEnum::Hour);
        assert!(hour.is_finite());
        let day_lunar: f32 = get_output_value(time, SystemTimeEnum::DayLunar);
        assert!(day_lunar.is_finite());
        let day_ordinal: f32 = get_output_value(time, SystemTimeEnum::DayOrdinal);
        assert!(day_ordinal.is_finite());
        let month: f32 = get_output_value(time, SystemTimeEnum::Month);
        assert!(month.is_finite());
        let year: f32 = get_output_value(time, SystemTimeEnum::Year);
        assert!(year.is_finite());

        assert_eq!(epoch, time.timestamp() as f32);
        assert_eq!(second, time.second() as f32);
        assert_eq!(minute, time.minute() as f32);
        assert_eq!(hour, time.hour() as f32);
        assert_eq!(day_lunar, time.day() as f32);
        assert_eq!(day_ordinal, time.ordinal() as f32);
        assert_eq!(month, time.month() as f32);
        assert_eq!(year, time.year() as f32);

        let epoch: f64 = get_output_value(time, SystemTimeEnum::Epoch);
        assert!(epoch.is_finite());
        let second: f64 = get_output_value(time, SystemTimeEnum::Second);
        assert!(second.is_finite());
        let minute: f64 = get_output_value(time, SystemTimeEnum::Minute);
        assert!(minute.is_finite());
        let hour: f64 = get_output_value(time, SystemTimeEnum::Hour);
        assert!(hour.is_finite());
        let day_lunar: f64 = get_output_value(time, SystemTimeEnum::DayLunar);
        assert!(day_lunar.is_finite());
        let day_ordinal: f64 = get_output_value(time, SystemTimeEnum::DayOrdinal);
        assert!(day_ordinal.is_finite());
        let month: f64 = get_output_value(time, SystemTimeEnum::Month);
        assert!(month.is_finite());
        let year: f64 = get_output_value(time, SystemTimeEnum::Year);
        assert!(year.is_finite());

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
    fn test_system_time_default_buffer_no_panic() {
        let block: SystemTimeBlock = SystemTimeBlock::default();
        assert_eq!(block.buffer(), 0.0);
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
        assert_eq!(block.buffer(), start_time.timestamp() as f64 + 42.0);
        assert_eq!(block.buffer(), output);
    }
}
