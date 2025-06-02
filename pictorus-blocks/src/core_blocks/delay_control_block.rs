use crate::traits::Scalar;
use core::time::Duration;
use pictorus_block_data::{BlockData as OldBlockData, FromPass};
use pictorus_traits::{Context, Matrix, Pass, PassBy, ProcessBlock};

/// Debounce or throttle an input signal.
///
/// Its output is a boolean, but for compatibility reasons it accepts many different scalar types
/// and determines their "truthiness" based on the scalar's value being non-zero
/// The block has two modes: Debounce and Throttle
///  - Debounce: Wait until the input signal stops being true for delay_time before emitting true.
///  - Throttle: Immediately emit true on first true input, but then wait delay_time before passing through a true input again.
pub struct DelayControlBlock<T: Apply> {
    pub data: OldBlockData,
    buffer: Option<T::Output>,
    /// This is the state of the block used for the debounce and throttle functionality
    state: T::State,
}

impl<T: Apply> Default for DelayControlBlock<T>
where
    OldBlockData: FromPass<T::Output>,
{
    fn default() -> Self {
        Self {
            data: <OldBlockData as FromPass<T::Output>>::from_pass(T::Output::default().as_by()),
            buffer: None,
            state: T::init_state(),
        }
    }
}

impl<T: Apply> ProcessBlock for DelayControlBlock<T>
where
    OldBlockData: FromPass<T::Output>,
{
    type Inputs = T;
    type Output = T::Output;
    type Parameters = Parameters;

    fn process<'b>(
        &'b mut self,
        parameters: &Self::Parameters,
        context: &dyn Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        let buffer = self.buffer.get_or_insert(T::Output::default());
        let output = T::apply(buffer, inputs, &mut self.state, parameters, context);
        self.data = <OldBlockData as FromPass<T::Output>>::from_pass(output);
        output
    }
}

pub trait Apply: Pass {
    type State;
    type Output: Pass + Default;

    fn init_state() -> Self::State;

    fn apply<'s>(
        store: &'s mut Self::Output,
        input: PassBy<Self>,
        state: &mut Self::State,
        parameters: &Parameters,
        context: &dyn Context,
    ) -> PassBy<'s, Self::Output>;
}

impl<S: Scalar> Apply for S {
    type State = Option<Duration>;
    type Output = f64;

    fn init_state() -> Self::State {
        None
    }

    fn apply<'s>(
        store: &'s mut Self::Output,
        input: PassBy<Self>,
        state: &mut Option<Duration>,
        parameters: &Parameters,
        context: &dyn Context,
    ) -> PassBy<'s, Self::Output> {
        let is_true = input.is_truthy();
        match parameters.method {
            DelayControlMethod::Debounce => {
                *store = debounce(is_true, state, parameters.delay, context.time());
            }

            DelayControlMethod::Throttle => {
                *store = throttle(is_true, state, parameters.delay, context.time());
            }
        }
        store.as_by()
    }
}

impl<S: Scalar, const NROWS: usize, const NCOLS: usize> Apply for Matrix<NROWS, NCOLS, S> {
    type Output = Matrix<NROWS, NCOLS, f64>;
    type State = [[Option<Duration>; NROWS]; NCOLS];

    fn init_state() -> Self::State {
        [[None; NROWS]; NCOLS]
    }

    fn apply<'s>(
        store: &'s mut Self::Output,
        input: PassBy<Self>,
        state: &mut Self::State,
        parameters: &Parameters,
        context: &dyn Context,
    ) -> PassBy<'s, Self::Output> {
        let input_flat = input.data.as_flattened();
        let state_flat = state.as_flattened_mut();
        let store_flat = store.data.as_flattened_mut();
        for i in 0..input_flat.len() {
            let is_true = input_flat[i].is_truthy();
            match parameters.method {
                DelayControlMethod::Debounce => {
                    store_flat[i] = debounce(
                        is_true,
                        &mut state_flat[i],
                        parameters.delay,
                        context.time(),
                    );
                }

                DelayControlMethod::Throttle => {
                    store_flat[i] = throttle(
                        is_true,
                        &mut state_flat[i],
                        parameters.delay,
                        context.time(),
                    );
                }
            }
        }
        store.as_by()
    }
}

/// If input is true set the state to Some(current_time) and the output to false
/// If input is false and the state is Some(d) and current_time - d >= delay then set the output store to true and the state to None
/// If input is false and the state is None set the output store to false
fn debounce(
    input: bool,
    state: &mut Option<Duration>,
    delay: Duration,
    curr_time: Duration,
) -> f64 {
    let mut output = false;
    if input {
        *state = Some(curr_time);
    } else if let Some(d) = state {
        if curr_time - *d >= delay {
            output = true;
            *state = None;
        }
    }
    if output {
        1.0
    } else {
        0.0
    }
}

/// If state is Some(d) and current_time - d >= delay then set state to None
/// then, if the input is true and state is None set the output store to true and the state to Some(current_time)
/// else set the output store to false
fn throttle(
    input: bool,
    state: &mut Option<Duration>,
    delay: Duration,
    curr_time: Duration,
) -> f64 {
    let mut output = false;
    if let Some(d) = state {
        if curr_time - *d >= delay {
            *state = None;
        }
    }
    if input && state.is_none() {
        output = true;
        *state = Some(curr_time);
    }
    if output {
        1.0
    } else {
        0.0
    }
}

#[derive(strum::EnumString, Clone, Copy, Debug)]
/// The method used to control the delay
pub enum DelayControlMethod {
    /// Debounce: Wait until the input signal stops being true for delay_time before emitting true.
    Debounce,
    /// Throttle: Emit true at most once every delay_time.
    Throttle,
}

#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    delay: Duration,
    method: DelayControlMethod,
}

impl Parameters {
    pub fn new(delay: f64, method: &str) -> Self {
        Self {
            delay: Duration::from_secs_f64(delay),
            method: method.parse().unwrap(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubRuntime;

    #[test]
    fn test_scalar_throttle() {
        let mut runtime = StubRuntime::default(); // Time is 0 timestep is 100ms
        let mut block = DelayControlBlock::<f64>::default();
        let parameters = Parameters::new(0.3, "Throttle");

        let output = block.process(&parameters, &runtime.context(), 0.0);
        assert!(!output.is_truthy());
        assert_eq!(block.data, OldBlockData::scalar_from_bool(false));

        runtime.tick(); // Time is 100ms
        let output = block.process(&parameters, &runtime.context(), 0.5);
        assert!(output.is_truthy());
        assert_eq!(block.data, OldBlockData::scalar_from_bool(true));

        runtime.tick(); // Time is 200ms
        let output = block.process(&parameters, &runtime.context(), 1.0);
        assert!(!output.is_truthy());
        assert_eq!(block.data, OldBlockData::scalar_from_bool(false));

        runtime.tick(); // Time is 300ms
        let output = block.process(&parameters, &runtime.context(), 1.5);
        assert!(!output.is_truthy());
        assert_eq!(block.data, OldBlockData::scalar_from_bool(false));

        runtime.tick(); // Time is 400ms
        let output = block.process(&parameters, &runtime.context(), 2.0);
        assert!(output.is_truthy());
        assert_eq!(block.data, OldBlockData::scalar_from_bool(true));

        runtime.tick(); // Time is 500ms
        let output = block.process(&parameters, &runtime.context(), 2.5);
        assert!(!output.is_truthy());
        assert_eq!(block.data, OldBlockData::scalar_from_bool(false));
    }

    #[test]
    fn test_scalar_debounce() {
        let mut runtime = StubRuntime::default(); // Time is 0 timestep is 100ms
        let mut block = DelayControlBlock::<f64>::default();
        let parameters = Parameters::new(0.3, "Debounce");

        // T= 0  we receive false
        let output = block.process(&parameters, &runtime.context(), 0.0);
        assert!(!output.is_truthy());
        assert_eq!(block.data, OldBlockData::scalar_from_bool(false));

        runtime.tick(); // T = 0.1s we receive true but still expect false
        let output = block.process(&parameters, &runtime.context(), -2.0);
        assert!(!output.is_truthy());
        assert_eq!(block.data, OldBlockData::scalar_from_bool(false));

        runtime.tick(); // T = 0.2s we receive false but still expect false until the delay cooldown is over
        let output = block.process(&parameters, &runtime.context(), 0.0);
        assert!(!output.is_truthy());
        assert_eq!(block.data, OldBlockData::scalar_from_bool(false));

        runtime.tick(); // T = 0.3s we receive false but still expect false until the delay cooldown is over
        let output = block.process(&parameters, &runtime.context(), 0.0);
        assert!(!output.is_truthy());
        assert_eq!(block.data, OldBlockData::scalar_from_bool(false));

        runtime.tick(); // T = 0.4s we receive false but expect true because delay cooldown is over
        let output = block.process(&parameters, &runtime.context(), 0.0);
        assert!(output.is_truthy());
        assert_eq!(block.data, OldBlockData::scalar_from_bool(true));

        runtime.tick(); // T = 0.5s we receive false and expect false since we already emitted true
        let output = block.process(&parameters, &runtime.context(), 0.0);
        assert!(!output.is_truthy());
        assert_eq!(block.data, OldBlockData::scalar_from_bool(false));

        runtime.tick(); // T = 0.6s we receive false and expect false since we already emitted true
        let output = block.process(&parameters, &runtime.context(), 0.0);
        assert!(!output.is_truthy());
        assert_eq!(block.data, OldBlockData::scalar_from_bool(false));

        // Show we can do it again
        runtime.tick(); // T = 0.7s we receive true but still expect false
        let output = block.process(&parameters, &runtime.context(), 1.0);
        assert!(!output.is_truthy());
        assert_eq!(block.data, OldBlockData::scalar_from_bool(false));

        runtime.tick(); // T = 0.8s we receive false setting the debounce cooldown in motion
        let output = block.process(&parameters, &runtime.context(), 0.0);
        assert!(!output.is_truthy());
        assert_eq!(block.data, OldBlockData::scalar_from_bool(false));

        // Fast forward to 0.3 seconds after the last true input
        runtime.context.time += Duration::from_secs_f64(0.3);
        let output = block.process(&parameters, &runtime.context(), 0.0);
        assert!(output.is_truthy());
        assert_eq!(block.data, OldBlockData::scalar_from_bool(true));

        runtime.tick();
        let output = block.process(&parameters, &runtime.context(), 0.0);
        assert!(!output.is_truthy());
        assert_eq!(block.data, OldBlockData::scalar_from_bool(false));
    }
}
