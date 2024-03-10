//! The library provides a bounded executor for tokio for a convenient way to run a fixed number of tasks concurrently
//! See: [`Rider::new`], [`Rider::spawn`] and [`Rider::shutdown`]

use std::future::Future;
use std::sync::Arc;

use tokio::sync::Semaphore;
use tokio::task::JoinSet;

/// Error returned from [`Rider::spawn`] function.
///
/// A spawn operation can fail only if the rider is off.
#[derive(Debug)]
pub struct RiderError(());

/// Task executor that maintains a maximum number of tasks running concurrently
///
/// # Example
///
/// ```rust
/// use rider::{Rider, RiderError};
///
/// #[tokio::main]
/// async fn main() -> Result<(), RiderError> {
///     let mut rider = Rider::new(10);
///     for _ in 0..100 {
///         rider
///             .spawn(async { /* do whatever you want */ })
///             .await?;
///     }
///     rider.shutdown().await;
///     Ok(())
/// }
/// ```
#[derive(Debug)]
pub struct Rider {
    sem: Arc<Semaphore>,
    set: JoinSet<()>,
}

impl RiderError {
    /// Instantiate [`RiderError`]
    fn closed() -> RiderError {
        RiderError(())
    }
}

impl Rider {
    /// Maximum number of tasks which a rider can hold. It is `usize::MAX >> 3`.
    ///
    /// Exceeding this limit typically results in a panic.
    pub const MAX_CAPACITY: usize = Semaphore::MAX_PERMITS;

    /// Creates a new rider with the given capacity.
    ///
    /// # Panics
    ///
    /// Panic if `capacity` is greater than [`Rider::MAX_CAPACITY`]
    ///
    /// # Example
    ///
    /// ```rust
    /// use rider::Rider;
    ///
    /// fn main() {
    ///     let rider = Rider::new(10);
    ///     // ...
    /// }
    /// ```
    pub fn new(capacity: usize) -> Rider {
        let sem = Arc::new(Semaphore::new(capacity));
        let set = JoinSet::new();
        Rider { sem, set }
    }

    /// Suspends until a seat is available and spawn the provided task on this [`Rider`].
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
    /// use rider::{Rider, RiderError};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), RiderError> {
    ///     let mut rider = Rider::new(10);
    ///     for _ in 0..100 {
    ///         rider.spawn(async move {
    ///             // Distribute your work
    ///         }).await?;
    ///     }
    ///     rider.shutdown().await;
    ///     Ok(())
    /// }
    /// ```
    pub async fn spawn<F>(&mut self, task: F) -> Result<(), RiderError>
    where
        F: Future<Output = ()>,
        F: Send + 'static,
    {
        let permit = self
            .sem
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| RiderError::closed())?;

        self.set.spawn(async move {
            task.await;
            drop(permit);
        });

        Ok(())
    }

    /// Closes the rider.
    /// This prevents calls to further [`Rider::spawn`] calls, and it waits for remaining tasks to complete.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rider::Rider;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut rider = Rider::new(10);
    ///     // ...
    ///     rider.shutdown().await;
    /// }
    /// ```
    pub async fn shutdown(mut self) {
        self.sem.close();
        while let Some(handle) = self.set.join_next().await {
            handle.expect("task in rider failed");
        }
    }
}
