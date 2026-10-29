use std::net::SocketAddr;

use clap::Parser;

/// Polykov server.
#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// Address to listen on.
    #[arg(long, default_value = "127.0.0.1:7700")]
    bind: SocketAddr,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let listener = tokio::net::TcpListener::bind(args.bind)
        .await
        .map_err(|e| std::io::Error::new(e.kind(), format!("cannot bind {}: {e}", args.bind)))?;
    eprintln!("polykov-server listening on {}", args.bind);
    axum::serve(listener, polykov_server::app::router()).await
}
