//! The library provides a bounded executor for tokio for a convenient way to run a fixed number of tasks concurrently
//! See: [`Driver::new`], [`Driver::spawn`] and [`Driver::shutdown`]

use std::future::Future;
use std::sync::Arc;

use tokio::sync::Semaphore;
use tokio::task::JoinSet;

/// Error returned from [`Driver::spawn`] function.
///
/// A spawn operation can fail only if the driver is off.
pub struct DriverError(());

/// Task executor that maintains a maximum number of tasks running concurrently
///
/// # Example
///
/// ```rust
/// use driver::{Driver, DriverError};
///
/// async fn main() -> Result<(), DriverError> {
///     let mut driver = Driver::new(10);
///     for _ in 0..100 {
///         driver
///             .spawn(async { /* do whatever you want */ })
///             .await?;
///     }
///     driver.shutdown().await;
///     Ok(())
/// }
/// ```
pub struct Driver {
    sem: Arc<Semaphore>,
    set: JoinSet<()>,
}

impl DriverError {
    /// Instantiate [`DriverError`]
    fn closed() -> DriverError {
        DriverError(())
    }
}

impl Driver {
    /// Maximum number of tasks which a driver can hold. It is `usize::MAX >> 3`.
    ///
    /// Exceeding this limit typically results in a panic.
    pub const MAX_CAPACITY: usize = Semaphore::MAX_PERMITS;

    /// Creates a new driver with the given capacity.
    ///
    /// # Panics
    ///
    /// Panic if `capacity` is greater than [`Driver::MAX_CAPACITY`]
    ///
    /// # Example
    ///
    /// ```rust
    /// use driver::Driver;
    ///
    /// fn main() {
    ///     let driver = Driver::new(10);
    ///     // ...
    /// }
    /// ```
    pub fn new(capacity: usize) -> Driver {
        let sem = Arc::new(Semaphore::new(capacity));
        let set = JoinSet::new();
        Driver { sem, set }
    }

    /// Suspends until a seat is available and spawn the provided task on this [`Driver`].
    ///
    /// The provided future will start running in the background once the function returns.
    ///
    /// # Cancel safety
    ///
    /// A [`Semaphore`] is used under the hood, which itself uses a queue to fairly distribute permits in the order they were requested.
    /// Cancelling a call to acquire_owned makes you lose your place in the queue.
    ///
    /// # Panics
    ///
    /// This method panics if called outside a Tokio runtime.
    ///
    /// # Example
    ///
    /// ```rust
    /// use driver::{Driver, DriverError};
    ///
    /// async fn main() -> Result<(), DriverError> {
    ///     let mut driver = Driver::new(10);
    ///     for _ in 0..100 {
    ///         driver.spawn(async move {
    ///             // Distribute your work
    ///         }).await?;
    ///     }
    ///     driver.shutdown().await;
    ///     Ok(())
    /// }
    /// ```
    pub async fn spawn<F>(&mut self, task: F) -> Result<(), DriverError>
    where
        F: Future<Output = ()>,
        F: Send + 'static,
    {
        let permit = self
            .sem
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| DriverError::closed())?;

        self.set.spawn(async move {
            task.await;
            drop(permit);
        });

        Ok(())
    }

    /// Closes the driver.
    /// This prevents calls to further [`Driver::spawn`] calls, and it waits for remaining tasks to complete.
    ///
    /// # Example
    ///
    /// ```rust
    /// use driver::Driver;
    ///
    /// async fn main() {
    ///     let mut driver = Driver::new(10);
    ///     // ...
    ///     driver.shutdown().await;
    /// }
    /// ```
    pub async fn shutdown(mut self) {
        self.sem.close();
        while let Some(handle) = self.set.join_next().await {
            handle.expect("task in driver failed");
        }
    }
}
