# rider

[![License](https://img.shields.io/badge/license-GNU_GPLv3-blue.svg)](https://choosealicense.com/licenses/gpl-3.0/)
[![Cargo](https://img.shields.io/crates/v/rider.svg)](https://crates.io/crates/rider)
[![Documentation](https://docs.rs/rider/badge.svg)](https://docs.rs/rider)

bounded executor for tokio; limit the count of tasks running

```rust
use rider::{Rider, RiderError};

#[tokio::main]
async fn main() -> Result<(), RiderError> {
    // create an executor that allows at most 10 task running concurrently
    let rider = Rider::new(10);

    for index in 0..10000 {
        rider
            .spawn(async move {
                println!("{}", index);
            })
            .await?; // Suspends until task is spawned
    }

    // Deny further tasks and join remaining tasks
    rider.shutdown().await;
}
```
