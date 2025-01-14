use axum::{
    Json, Router,
    http::StatusCode,
    routing::{get, post},
};
use clap::{Parser, Subcommand};
use client::client_main;
use server::server_main;

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
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Server { port } => server_main(port).await,
        Commands::Client { url } => client_main(url).await,
    }
}
