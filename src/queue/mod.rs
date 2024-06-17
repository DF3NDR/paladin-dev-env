// src/queue/mod.rs
use std::sync::{Arc, Mutex};
use tokio::task;
use crate::analyzer::llm_analyzer::analyze_data;
use crate::config::Settings;
use crate::data::NormalizedData;

lazy_static::lazy_static! {
    static ref ANALYSIS_QUEUE: Mutex<Vec<NormalizedData>> = Mutex::new(Vec::new());
}

pub async fn start_analyzer(config: Arc<Settings>) {
    loop {
        let task = {
            let mut queue = ANALYSIS_QUEUE.lock().unwrap();
            queue.pop()
        };

        if let Some(data) = task {
            let config = config.clone(); // Clone config for each task
            task::spawn(async move {
                // Call analyze function
                let result = analyze_data(&data.title, "Your prompt here", &config).await;
                // Handle result here
                match result {
                    Ok(summary) => {
                        println!("Analysis successful: {:?}", summary);
                    }
                    Err(e) => {
                        eprintln!("Analysis failed: {:?}", e);
                    }
                }
            });
        } else {
            // Sleep for a short duration to prevent busy-waiting
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }
}

pub fn enqueue_for_analysis(data: NormalizedData) {
    let mut queue = ANALYSIS_QUEUE.lock().unwrap();
    queue.push(data);
}
