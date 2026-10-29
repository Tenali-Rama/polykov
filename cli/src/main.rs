use std::process::ExitCode;

use clap::{Parser, Subcommand};
use serde_json::json;

mod client;
mod cmd;
mod config;
mod error;

/// Polykov command-line client.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Server URL [default: $POLYKOV_SERVER, then the CLI config, then http://127.0.0.1:7700]
    #[arg(long, global = true, value_name = "URL")]
    server: Option<String>,

    /// Print one JSON object instead of text (errors too)
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Log in and store the session token (password from $POLYKOV_PASSWORD or a prompt)
    Login,
    /// End the session and remove the stored token
    Logout,
    /// Show server version, database state and whether the stored token is valid
    Status,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = cmd::Ctx::load(cli.server).and_then(|ctx| match cli.command {
        Command::Login => cmd::login::run(&ctx),
        Command::Logout => cmd::logout::run(&ctx),
        Command::Status => cmd::status::run(&ctx),
    });
    match result {
        Ok(out) if cli.json => println!("{}", out.json),
        Ok(out) => println!("{}", out.human),
        Err(e) if cli.json => {
            eprintln!("{}", json!({ "error": e.to_string() }));
            return ExitCode::FAILURE;
        }
        Err(e) => {
            eprintln!("polykov: {e}");
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}
