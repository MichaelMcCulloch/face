use std::{path::PathBuf, sync::Arc};

use actix_rt::signal;
use actix_web::rt;
use clap::Parser;
use engine::IndexEngine;

use server::run_server;
use tokio::sync::{oneshot, Mutex};

mod api;
mod engine;
mod index;
mod server;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    index_path: PathBuf,
    #[arg( long, default_value_t = String::from("0.0.0.0"))]
    pub(crate) host: String,
    #[arg(long, default_value_t = 6947)]
    pub(crate) port: u16,
    #[arg(long, default_value_t = 16)]
    pub(crate) max_parallelism: u16,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let args = Args::parse();
    let index_path = args.index_path;

    let faiss_index = index::FaissIndex::new(&index_path)?;

    let system_runner = rt::System::new();

    let (sender, receiver): (oneshot::Sender<()>, oneshot::Receiver<()>) = oneshot::channel::<()>();

    let receiver = Arc::new(Mutex::new(receiver));
    let exec = async {
        rt::spawn(async move {
            signal::ctrl_c()
                .await
                .expect("Failed to listen for exit signal");
            let _ = sender.send(());
        });

        let index_engine = IndexEngine::new(faiss_index, receiver, args.max_parallelism).await;
        let _ = run_server(index_engine, args.host, args.port)
            .unwrap()
            .await;
    };

    system_runner.block_on(exec);
    Ok(())
}
