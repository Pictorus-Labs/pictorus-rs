//! This crate provides structs used for unit testing blocks that implement Corelib traits.
//!
//! Currently the [`StubContext`] and [`StubRuntime`] structs are provided. The [`StubContext`] struct
//! implements the [`pictorus_traits::Context`] trait and can be used in unit tests to be able to make
//! calls against corelib block functionality. The [`StubRuntime`] struct wraps the [`StubContext`] struct
//! and offers a convenient way to simulate the passage of time in a unit test.
//!
//! This crate should be considered unstable and only used as a development dependency.

extern crate std;
use derive_new::new;
use pictorus_traits::Context;
use std::time::Duration;

/// An implementation of the [`pictorus_traits::Context`] trait that can be used in unit tests.
#[derive(Debug, Copy, Clone, new)]
pub struct StubContext {
    pub time: Duration,
    pub timestep: Option<Duration>,
    pub fundamental_timestep: Duration,
}

impl Default for StubContext {
    fn default() -> Self {
        Self::new(Duration::from_secs(0), None, Duration::from_millis(100))
    }
}

impl Context for StubContext {
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

/// A struct that wraps a [`StubContext`] and provides a convenient way to simulate the passage of time in a unit test.
#[derive(Debug, new, Clone, Copy, Default)]
pub struct StubRuntime {
    pub context: StubContext,
}

impl StubRuntime {
    pub fn tick(&mut self) {
        self.context.time += self.context.fundamental_timestep;
        self.context.timestep = Some(self.context.fundamental_timestep);
    }

    pub fn context(&self) -> StubContext {
        self.context
    }

    pub fn set_time(&mut self, time: Duration) {
        self.context.time = time;
    }
}
