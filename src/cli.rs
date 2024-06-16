// src/cli.rs
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "smartcontent-aggregator")]
pub struct Cli {
    #[structopt(short, long)]
    pub config: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_args() {
        let args = Cli::from_iter(&["smartcontent-aggregator", "--config", "config.yaml"]);
        assert_eq!(args.config, "config.yaml");
    }
}
