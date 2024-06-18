use std::time::Duration;
use tokio::time::interval;

pub async fn start_scheduler() {
    let mut interval = interval(Duration::from_secs(60)); // Runs every minute

    loop {
        interval.tick().await;
        println!("Scheduler task running...");
        // Add task logic here
    }
}
