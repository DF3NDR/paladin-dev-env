/*
Scheduler

This module is responsible for running the scheduler task. The scheduler is a background
service that is not internal to the system.  It is part of the orchestration system
that runs Jobs if they are scheduled for regular intervals (as opposed to being triggered
by an event, queued, or run on-demand).

The scheduler itself is part of the infrastructure layer. On a standalone system it would
be a cron job or a Windows Task Scheduler task. In a distributed system it would be a
service running on a separate server or container.

A schedule itself contains compositions (work_orders) that are run at regular intervals or 
scheduled to run at a particular time.
*/
use std::time::Duration;
use tokio::time::interval;

pub async fn start_scheduler() {
    let mut interval = interval(Duration::from_secs(60)); // Runs every minute

    loop {
        interval.tick().await;
        println!("Scheduler task running...");
        // add logic here
    }
}
