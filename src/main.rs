use smartcontent_aggregator::*;
use structopt::StructOpt;
use actix_web::{web, App, HttpServer};
use std::sync::Mutex;
use log::info;
use std::sync::Arc;
use env_logger::Env;

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
    // let config = Settings::load_from_file(&opt.config).expect("Failed to load configuration");
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

    // Clone config for use in the closure
    let server_config = config.clone(); // Ensure Settings implements Clone

    // Use Arc to avoid multiple clones if the clone is expensive
    let config = Arc::new(config);

    HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .app_data(web::Data::new(config.clone())) // Use Arc::clone, which is cheap
            .configure(router::configure) // Configure routes
    })
    .bind(format!("{}:{}", server_config.server.host, server_config.server.port))?
    .run()
    .await
}
