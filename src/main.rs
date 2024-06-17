// src/main.rs
use smartcontent_aggregator::*;
use structopt::StructOpt;
use actix_web::{web, App, HttpServer};
use std::sync::Mutex;
use log::info;
use std::sync::Arc;
use env_logger::Env;
use tokio::task;
use smartcontent_aggregator::scheduler::start_scheduler;
use smartcontent_aggregator::queue::start_analyzer;

#[derive(StructOpt, Debug)]
#[structopt(name = "smartcontent-aggregator")]
struct Opt {
    #[structopt(short, long, default_value = "config.yml")]
    config: String,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    let opt = Opt::from_args();
    let config = match Settings::load_from_file(&opt.config) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to load configuration: {:?}", e);
            std::process::exit(1);
        }
    };
    
    info!("Loaded configuration: {:?}", config);

    let data = web::Data::new(AppState {
        summaries: Mutex::new(Vec::new()),
    });

    let server_config = config.clone();
    let config = Arc::new(config);

    // Start scheduler and analyzer
    task::spawn(start_scheduler());
    task::spawn(start_analyzer(config.clone()));

    HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .app_data(web::Data::new(config.clone()))
            .configure(router::configure)
    })
    .bind(format!("{}:{}", server_config.server.host, server_config.server.port))?
    .run()
    .await
}
