use structopt::StructOpt;
use env_logger::Env;
use log::info;
use smartcontent_aggregator::config::Settings;
use smartcontent_aggregator::setup::setup_and_run;

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

    setup_and_run(config).await
}
