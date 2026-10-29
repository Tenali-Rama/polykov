use std::process::ExitCode;

use clap::{Parser, Subcommand};
use serde_json::json;

use cmd::object::flag::Flag;

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
    /// Create an object and print its id
    New(cmd::object::new::Args),
    /// Show one object
    Show { id: u64 },
    /// List objects (unarchived by default), pinned first
    Ls(cmd::object::ls::Args),
    /// Change an object's title, tags or properties
    Edit(cmd::object::edit::Args),
    /// Move an object to the trash
    Rm { id: u64 },
    /// Pin an object
    Pin { id: u64 },
    /// Unpin an object
    Unpin { id: u64 },
    /// Archive an object
    Archive { id: u64 },
    /// Unarchive an object
    Unarchive { id: u64 },
    /// Add, remove or list links between objects
    #[command(subcommand)]
    Link(cmd::link::Cmd),
    /// Add, list, edit or delete comments on an object
    #[command(subcommand)]
    Comment(cmd::comment::Cmd),
    /// List, restore or permanently delete trashed objects
    #[command(subcommand)]
    Trash(cmd::trash::Cmd),
    /// Upload, list, download or delete an object's attached files
    #[command(subcommand)]
    Attach(cmd::attach::Command),
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = cmd::Ctx::load(cli.server).and_then(|ctx| match cli.command {
        Command::Login => cmd::login::run(&ctx),
        Command::Logout => cmd::logout::run(&ctx),
        Command::Status => cmd::status::run(&ctx),
        Command::New(args) => cmd::object::new::run(&ctx, args),
        Command::Show { id } => cmd::object::show::run(&ctx, id),
        Command::Ls(args) => cmd::object::ls::run(&ctx, args),
        Command::Edit(args) => cmd::object::edit::run(&ctx, args),
        Command::Rm { id } => cmd::object::rm::run(&ctx, id),
        Command::Pin { id } => cmd::object::flag::run(&ctx, id, Flag::Pinned, true),
        Command::Unpin { id } => cmd::object::flag::run(&ctx, id, Flag::Pinned, false),
        Command::Archive { id } => cmd::object::flag::run(&ctx, id, Flag::Archived, true),
        Command::Unarchive { id } => cmd::object::flag::run(&ctx, id, Flag::Archived, false),
        Command::Link(cmd) => cmd::link::run(&ctx, cmd),
        Command::Comment(cmd) => cmd::comment::run(&ctx, cmd),
        Command::Trash(cmd) => cmd::trash::run(&ctx, cmd),
        Command::Attach(command) => cmd::attach::run(&ctx, command),
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
