use pictorus_block_data::BlockData as OldBlockData;
use pictorus_traits::{PassBy, ProcessBlock, Scalar};

#[derive(strum::EnumString)]
pub enum Method {
    CountDown,
    StopWatch,
}

/// Parameters for the TimerBlock
pub struct Parameters {
    /// Method of the TimerBlock: CountDown or StopWatch. CountDown will count down from
    /// the countdown_time_s, while StopWatch will count up from 0.
    pub method: Method,
    /// If the TimerBlock is interruptable. If this is true and the trigger is > 0, the timer will restart.
    pub interruptable: bool,
    /// The time in seconds to count down from. Only used if method is CountDown.
    pub countdown_time_s: f64,
}

impl Parameters {
    pub fn new(method: &str, interruptable: bool, countdown_time_s: f64) -> Parameters {
        Parameters {
            method: method.parse().expect("Faile to parse Timer Method"),
            interruptable,
            countdown_time_s,
        }
    }
}

/// The Timer block allows timekeeping around discrete events - either by Stopwatch mode or Countdown mode.
///
/// The input signal serves as the trigger to start the timer. Any input which is "True" will commence the timer.
/// If the Interruptible option is enabled, the timer will reset every iteration where the input is True.
/// Otherwise, the timer will commence on the first True value. It will then either count down and reset at zero,
/// or count up without ever restarting.
///
/// This block is useful for tracking how much time has passed since an event for logical conditions.
pub struct TimerBlock<T> {
    pub data: OldBlockData,
    buffer: T,
    timer_running: bool,
    start_time_s: T,
}

impl<T> Default for TimerBlock<T>
where
    T: Scalar + num_traits::Zero,
{
    fn default() -> Self {
        Self {
            data: OldBlockData::from_scalar(0.0),
            buffer: T::zero(),
            timer_running: false,
            start_time_s: T::zero(),
        }
    }
}

impl TimerBlock<f64> {
    fn _do_countdown(&mut self, time_since_start: f64, countdown_time_s: f64) {
        if time_since_start < countdown_time_s {
            self.buffer = countdown_time_s - time_since_start;
        } else {
            self.buffer = 0.0;
            self.timer_running = false;
        }
    }

    fn _do_stopwatch(&mut self, time_since_start: f64) {
        self.buffer = time_since_start;
    }
}

impl ProcessBlock for TimerBlock<f64> {
    type Inputs = f64;
    type Output = f64;
    type Parameters = Parameters;

    fn process(
        &mut self,
        parameters: &Self::Parameters,
        context: &dyn pictorus_traits::Context,
        input: PassBy<Self::Inputs>,
    ) -> PassBy<'_, Self::Output> {
        let time = context.time().as_secs_f64();

        let trigger_high = input > 0.0;
        // Early exit if not running and input trigger is false
        if !self.timer_running && !trigger_high {
            self.data.set_scalar(self.buffer);
            return self.buffer;
        }

        if trigger_high {
            if !self.timer_running {
                // Start the timer
                self.start_time_s = time;
                self.timer_running = true;
            } else if self.timer_running && parameters.interruptable {
                // Interrupt and restart the timer
                self.start_time_s = time;
            }
        }

        let time_since_start = time - self.start_time_s;

        match parameters.method {
            Method::CountDown => {
                self._do_countdown(time_since_start, parameters.countdown_time_s);
            }
            Method::StopWatch => {
                self._do_stopwatch(time_since_start);
            }
        }

        self.data.set_scalar(self.buffer);
        self.buffer
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::StubRuntime;
    use core::time;

    use super::*;

    #[test]
    fn test_countdown_timer_non_interruptable() {
        let mut runtime = StubRuntime::default();
        let p = Parameters::new("CountDown", false, 5.0);
        let mut block = TimerBlock::<f64>::default();

        let output = block.process(&p, &runtime.context(), 0.0);
        assert_eq!(block.data.scalar(), 0.0);
        assert_eq!(output, 0.0);

        runtime.set_time(time::Duration::from_secs_f64(1.0));
        let output = block.process(&p, &runtime.context(), 1.0);
        assert_eq!(block.data.scalar(), 5.0);
        assert_eq!(output, 5.0);

        runtime.set_time(time::Duration::from_secs_f64(2.0));
        let output = block.process(&p, &runtime.context(), 0.0);
        assert_eq!(block.data.scalar(), 4.0);
        assert_eq!(output, 4.0);

        // Countdown not interrupted
        runtime.set_time(time::Duration::from_secs_f64(3.0));
        let output = block.process(&p, &runtime.context(), 1.0);
        assert_eq!(block.data.scalar(), 3.0);
        assert_eq!(output, 3.0);

        runtime.set_time(time::Duration::from_secs_f64(10.0));
        let output = block.process(&p, &runtime.context(), 0.0);
        assert_eq!(block.data.scalar(), 0.0);
        assert_eq!(output, 0.0);

        runtime.set_time(time::Duration::from_secs_f64(11.0));
        let output = block.process(&p, &runtime.context(), 0.0);
        assert_eq!(block.data.scalar(), 0.0);
        assert_eq!(output, 0.0);
    }

    #[test]
    fn test_countdown_timer_interruptable() {
        let mut runtime = StubRuntime::default();
        let p = Parameters::new("CountDown", true, 5.0);
        let mut block = TimerBlock::<f64>::default();

        // Timer hasn't started
        runtime.set_time(time::Duration::from_secs_f64(1.0));
        let output = block.process(&p, &runtime.context(), 0.0);
        assert_eq!(block.data.scalar(), 0.0);
        assert_eq!(output, 0.0);

        // Timer started, should be at countdown_time_s
        runtime.set_time(time::Duration::from_secs_f64(2.0));
        let output = block.process(&p, &runtime.context(), 1.0);
        assert_eq!(block.data.scalar(), 5.0);
        assert_eq!(output, 5.0);

        runtime.set_time(time::Duration::from_secs_f64(3.0));
        let output = block.process(&p, &runtime.context(), 0.0);
        assert_eq!(block.data.scalar(), 4.0);
        assert_eq!(output, 4.0);

        // Countdown interrupted, resets
        runtime.set_time(time::Duration::from_secs_f64(4.0));
        let output = block.process(&p, &runtime.context(), 1.0);
        assert_eq!(block.data.scalar(), 5.0);
        assert_eq!(output, 5.0);

        // Countdown interrupted, resets
        runtime.set_time(time::Duration::from_secs_f64(5.0));
        let output = block.process(&p, &runtime.context(), 1.0);
        assert_eq!(block.data.scalar(), 5.0);
        assert_eq!(output, 5.0);

        // Countdown resumes
        runtime.set_time(time::Duration::from_secs_f64(6.0));
        let output = block.process(&p, &runtime.context(), 0.0);
        assert_eq!(block.data.scalar(), 4.0);
        assert_eq!(output, 4.0);
    }

    #[test]
    fn test_stopwatch_timer_non_interruptable() {
        let mut runtime = StubRuntime::default();
        let p = Parameters::new("StopWatch", false, 5.0);
        let mut block = TimerBlock::<f64>::default();

        // Timer hasn't started
        runtime.set_time(time::Duration::from_secs_f64(1.0));
        let output = block.process(&p, &runtime.context(), 0.0);
        assert_eq!(block.data.scalar(), 0.0);
        assert_eq!(output, 0.0);

        // Timer started, should be at time since start
        runtime.set_time(time::Duration::from_secs_f64(2.0));
        let output = block.process(&p, &runtime.context(), 1.0);
        assert_eq!(block.data.scalar(), 0.0);
        assert_eq!(output, 0.0);

        runtime.set_time(time::Duration::from_secs_f64(3.0));
        let output = block.process(&p, &runtime.context(), 0.0);
        assert_eq!(block.data.scalar(), 1.0);
        assert_eq!(output, 1.0);

        // StopWatch not interrupted
        runtime.set_time(time::Duration::from_secs_f64(4.0));
        let output = block.process(&p, &runtime.context(), 1.0);
        assert_eq!(block.data.scalar(), 2.0);
        assert_eq!(output, 2.0);

        runtime.set_time(time::Duration::from_secs_f64(10.0));
        let output = block.process(&p, &runtime.context(), 0.0);
        assert_eq!(block.data.scalar(), 8.0);
        assert_eq!(output, 8.0);

        runtime.set_time(time::Duration::from_secs_f64(100.0));
        let output = block.process(&p, &runtime.context(), 0.0);
        assert_eq!(block.data.scalar(), 98.0);
        assert_eq!(output, 98.0);
    }

    #[test]
    fn test_stopwatch_timer_interruptable() {
        let mut runtime = StubRuntime::default();
        let p = Parameters::new("StopWatch", true, 5.0);
        let mut block = TimerBlock::<f64>::default();

        // Timer hasn't started
        runtime.set_time(time::Duration::from_secs_f64(1.0));
        let output = block.process(&p, &runtime.context(), 0.0);
        assert_eq!(block.data.scalar(), 0.0);
        assert_eq!(output, 0.0);

        // Timer started, should be at time since start
        runtime.set_time(time::Duration::from_secs_f64(2.0));
        let output = block.process(&p, &runtime.context(), 1.0);
        assert_eq!(block.data.scalar(), 0.0);
        assert_eq!(output, 0.0);

        runtime.set_time(time::Duration::from_secs_f64(3.0));
        let output = block.process(&p, &runtime.context(), 0.0);
        assert_eq!(block.data.scalar(), 1.0);
        assert_eq!(output, 1.0);

        // StopWatch interrupted
        runtime.set_time(time::Duration::from_secs_f64(4.0));
        let output = block.process(&p, &runtime.context(), 1.0);
        assert_eq!(block.data.scalar(), 0.0);
        assert_eq!(output, 0.0);

        // StopWatch resumes
        runtime.set_time(time::Duration::from_secs_f64(10.0));
        let output = block.process(&p, &runtime.context(), 0.0);
        assert_eq!(block.data.scalar(), 6.0);
        assert_eq!(output, 6.0);

        runtime.set_time(time::Duration::from_secs_f64(100.0));
        let output = block.process(&p, &runtime.context(), 0.0);
        assert_eq!(block.data.scalar(), 96.0);
        assert_eq!(output, 96.0);
    }
}
