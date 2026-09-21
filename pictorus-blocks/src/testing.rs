//! This crate provides structs used for unit testing blocks that implement Corelib traits.
//!
//! Currently the [`StubModelClock`] and [`StubRuntime`] structs are provided. The [`StubModelClock`] struct
//! implements the [`pictorus_traits::ModelClock`] trait and can be used in unit tests to be able to make
//! calls against corelib block functionality. The [`StubRuntime`] struct wraps the [`StubModelClock`] struct
//! and offers a convenient way to simulate the passage of time in a unit test.
//!
//! This crate should be considered unstable and only used as a development dependency.

extern crate std;
use derive_new::new;
use pictorus_traits::ModelClock;
use std::time::Duration;

/// An implementation of the [`pictorus_traits::ModelClock`] trait that can be used in unit tests.
#[derive(Debug, Copy, Clone, new)]
pub struct StubModelClock {
    pub time: Duration,
    pub timestep: Option<Duration>,
    pub fundamental_timestep: Duration,
}

impl Default for StubModelClock {
    fn default() -> Self {
        Self::new(Duration::from_secs(0), None, Duration::from_millis(100))
    }
}

impl ModelClock for StubModelClock {
    fn time(&self) -> Duration {
        self.time
    }

    fn timestep(&self) -> Option<Duration> {
        self.timestep
    }

    fn fundamental_timestep(&self) -> Duration {
        self.fundamental_timestep
    }
}

/// A struct that wraps a [`StubModelClock`] and provides a convenient way to simulate the passage of time in a unit test.
#[derive(Debug, new, Clone, Copy, Default)]
pub struct StubRuntime {
    pub model_clock: StubModelClock,
}

impl StubRuntime {
    pub fn tick(&mut self) {
        self.model_clock.time += self.model_clock.fundamental_timestep;
        self.model_clock.timestep = Some(self.model_clock.fundamental_timestep);
    }

    pub fn model_clock(&self) -> StubModelClock {
        self.model_clock
    }

    pub fn set_time(&mut self, time: Duration) {
        self.model_clock.time = time;
    }
}
