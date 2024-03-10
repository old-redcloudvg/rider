# driver

bounded executor for tokio; limit the count of tasks running

```rust
use driver::{Driver, DriverError};

#[tokio::main]
async fn main() -> Result<(), DriverError> {
    let driver = Driver::new(10);

    for index in 0.10000 {
        driver
            .spawn(async move {
                println!("{}", index);
            })
            .await?;
    }

    // Deny further tasks and join remaining tasks
    driver.shutdown().await;
}
```
