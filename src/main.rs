use crate::backup::start_backup;
use crate::opts::Opts;
use clap::Clap;
use simple_logger::SimpleLogger;

mod attachment_type;
mod backup;
mod connector;
mod context;
mod in_progress;
mod opts;
mod types;

#[tokio::main]
async fn main() {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Debug)
        .init()
        .unwrap();

    let opts: Opts = Opts::parse();

    start_backup(opts).await;
}
