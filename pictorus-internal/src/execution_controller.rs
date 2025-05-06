/// This controller is used to determine when a component should execute based on a desired
/// frequency. That is once every N times [ExecutionController::should_execute()] is called
/// when the controller is constructed with a limit of N.
///
/// `ExecutionController` keeps track of execution attempts and returns `true` only
/// when the counter is at zero. After each call to `should_execute()`, the counter
/// increments until it reaches the specified limit, at which point it resets to zero.
///
/// The controller can be constructed with a limit and a count, or just a limit. If a count
/// is provided, the controller will start at that count and increment from there. This can
/// be used to stagger the execution of multiple components that share the same limit.
///
/// # Examples
///
/// ```
/// use pictorus_internal::ExecutionController;
/// let mut controller = ExecutionController::with_limit(5); // Run once every 5 times
/// assert!(controller.should_execute());  // First call returns true
/// assert!(!controller.should_execute()); // Next 4 calls return false
/// assert!(!controller.should_execute());
/// assert!(!controller.should_execute());
/// assert!(!controller.should_execute());
/// assert!(controller.should_execute());  // Cycle repeats
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExecutionController {
    /// [ExecutionController::should_execute()] will return true once every `limit` calls
    limit: usize,
    /// The current count of calls to [ExecutionController::should_execute()] between 0 and `limit`-1
    count: usize,
}

impl ExecutionController {
    /// Create a new `ExecutionController` with the specified limit and count
    pub fn new(limit: usize, count: usize) -> Self {
        Self { limit, count }
    }

    /// Create a new `ExecutionController` with the specified limit and a count defaulting to 0
    pub fn with_limit(limit: usize) -> Self {
        Self { limit, count: 0 }
    }
}

impl ExecutionController {
    /// This is the main function of the controller. It returns `true` once every `limit` calls
    /// and `false` otherwise. The controller keeps track of the number of calls and resets the
    /// count to zero once it reaches the limit.
    ///
    /// # Examples
    /// ```
    /// use pictorus_internal::ExecutionController;
    /// let mut controller = ExecutionController::with_limit(5); // Run once every 5 times
    /// let mut run_count = 0;
    /// for _ in 0..100 {
    ///     if controller.should_execute() {
    ///        // This will run once every 5 iterations
    ///        println!("Executing");
    ///       run_count += 1;
    ///    }
    /// }
    /// assert_eq!(run_count, 20);
    /// ```
    pub fn should_execute(&mut self) -> bool {
        let output = self.count == 0;
        self.count += 1;
        if self.count >= self.limit {
            self.count = 0;
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_execution_controller() {
        let mut controller = ExecutionController::new(5, 0); //Run once every 5 times
        assert!(controller.should_execute());
        assert!(!controller.should_execute());
        assert!(!controller.should_execute());
        assert!(!controller.should_execute());
        assert!(!controller.should_execute());
        assert!(controller.should_execute());
        assert!(!controller.should_execute());
        assert!(!controller.should_execute());
        assert!(!controller.should_execute());
        assert!(!controller.should_execute());
        assert!(controller.should_execute());
    }

    #[test]
    fn test_new() {
        assert_eq!(
            ExecutionController::new(42, 9),
            ExecutionController {
                limit: 42,
                count: 9
            }
        )
    }

    #[test]
    fn test_with_limit() {
        assert_eq!(
            ExecutionController::with_limit(5),
            ExecutionController::new(5, 0)
        )
    }

    #[test]
    fn test_pathological_count() {
        let mut controller = ExecutionController::new(5, 1337);
        // Should on first iteration return count to zero and get back to a sane state
        assert!(!controller.should_execute());
        assert_eq!(controller, ExecutionController::new(5, 0));
    }

    #[test]
    fn test_pathological_zero_limit() {
        let mut controller = ExecutionController::new(0, 0);
        // Should always return true
        assert_eq!(controller, ExecutionController { limit: 0, count: 0 });
        assert!(controller.should_execute());
        assert_eq!(controller, ExecutionController { limit: 0, count: 0 });
        assert!(controller.should_execute());
    }
}
