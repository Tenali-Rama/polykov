use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use polykov_server::config::{self, Flags};
use polykov_server::db::Db;

/// Polykov server.
#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// Data dir [env: POLYKOV_DATA_DIR] [default: $XDG_DATA_HOME/polykov or ~/.local/share/polykov]
    #[arg(long)]
    data_dir: Option<PathBuf>,
    /// Address to listen on [env: POLYKOV_BIND] [default: 127.0.0.1:7700]
    #[arg(long)]
    bind: Option<SocketAddr>,
    /// Directory with the built web UI [env: POLYKOV_WEB_DIR] [default: web/dist]
    #[arg(long)]
    web_dir: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> ExitCode {
    match run(Args::parse()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("polykov-server: {e}");
            ExitCode::FAILURE
        }
    }
}

async fn run(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    let flags = Flags {
        data_dir: args.data_dir,
        bind: args.bind,
        web_dir: args.web_dir,
    };
    let cfg = config::load(flags, |key| std::env::var(key).ok())?;
    let db = Db::open(&cfg.data_dir)?;
    polykov_server::trash::spawn_purger(db.clone())?;
    let listener = tokio::net::TcpListener::bind(cfg.bind)
        .await
        .map_err(|e| format!("cannot bind {}: {e}", cfg.bind))?;
    eprintln!(
        "polykov-server listening on {} (data dir {})",
        cfg.bind,
        cfg.data_dir.display()
    );
    axum::serve(listener, polykov_server::app::router(db, &cfg.web_dir)).await?;
    Ok(())
}
