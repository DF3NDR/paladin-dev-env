use std::sync::{Arc, Mutex};
use tokio::task;
use crate::config::Settings;
use crate::domain::entities::content_item::ContentItem;

// lazy_static::lazy_static! {
//     static ref ANALYSIS_QUEUE: Mutex<Vec<ContentItem>> = Mutex::new(Vec::new());
// }

// pub async fn start_queue(config: Arc<Settings>) {
//     loop {
//         let task = {
//             let mut queue = ANALYSIS_QUEUE.lock().unwrap();
//             queue.pop()
//         };

//         if let Some(data) = task {
//             let config = config.clone();
//             task::spawn(async move {
//                 // Analyze the data here
//                 println!("Analyzing data: {:?}", data);
//                 // Implement the analysis logic here
//             });
//         } else {
//             tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
//         }
//     }
// }

// pub fn enqueue_for_analysis(data: ContentItem) {
//     let mut queue = ANALYSIS_QUEUE.lock().unwrap();
//     queue.push(data);
// }
