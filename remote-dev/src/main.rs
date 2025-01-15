use std::path::PathBuf;

use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use clap::{Args, Parser, Subcommand};
use client::client_main;
use server::server_main;
use strum::EnumString;

mod client;
mod server;

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Server {
        #[arg(short, long, default_value_t = 3000)]
        port: u16,
    },
    Client {
        #[arg(short, long)]
        url: String,
        #[command(subcommand)]
        command: ClientCommand,
    },
}

#[derive(Subcommand, Clone, EnumString)]
enum ClientCommand {
    Upload {
        #[arg(short, long)]
        file: PathBuf,
    },
    Execute {
        #[arg(short, long)]
        file: PathBuf,
    },
    Run {
        #[arg(short, long)]
        file: PathBuf,
    },
    Kill {
        #[arg(short, long)]
        pid: u32,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Server { port } => server_main(port).await,
        Commands::Client { url, command } => match client_main(url, command).await {
            Ok(s) => println!("{}", s),
            Err(e) => println!("Error: {}", e),
        },
    }
}
