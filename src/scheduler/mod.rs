// src/scheduler/mod.rs
use std::time::{Duration, Instant};
use tokio::time::interval;

pub async fn start_scheduler() {
    let mut interval = interval(Duration::from_secs(60)); // Runs every minute

    loop {
        interval.tick().await;
        // Call the function to trigger fetching data
        println!("Triggering fetch task...");
        // Add your fetch logic here or call the fetch function
    }
}
