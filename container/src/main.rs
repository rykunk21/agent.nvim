mod llm;
mod mcp;
mod spec;
mod communication;
mod config;
mod utils;
mod command;
mod file_ops;

use anyhow::Result;
use log::info;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    info!("Starting MCP orchestration layer");

    // Get configuration from environment
    let grpc_port = env::var("GRPC_PORT")
        .unwrap_or_else(|_| "50051".to_string())
        .parse::<u16>()?;

    let grpc_addr = format!("0.0.0.0:{}", grpc_port).parse()?;

    info!("Starting gRPC server on {}", grpc_addr);

    // Start the gRPC server
    communication::grpc::start_server(grpc_addr).await?;

    Ok(())
}
