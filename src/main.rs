use std::io::{self, BufRead, BufReader};
use log::{info, error};
use agent_nvim::communication::{JsonProtocol, MessageHandler};

fn main() {
    // Initialize logging to stderr so it doesn't interfere with JSON communication
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .target(env_logger::Target::Stderr)
        .init();

    info!("Starting agent.nvim binary");

    // Initialize message handler and protocol
    let mut handler = match MessageHandler::new() {
        Ok(h) => h,
        Err(e) => {
            error!("Failed to initialize message handler: {}", e);
            return;
        }
    };
    
    let mut protocol = JsonProtocol::new();

    // Send startup confirmation
    let startup_msg = agent_nvim::communication::Message::new_notification(
        agent_nvim::communication::MessagePayload::Startup {
            status: "ready".to_string(),
        }
    );
    
    if let Err(e) = protocol.send_message(startup_msg) {
        error!("Failed to send startup message: {}", e);
        return;
    }

    // Message handling loop
    if let Err(e) = protocol.read_messages(|message| {
        handler.handle_message(message)
    }) {
        error!("Message loop error: {}", e);
    }

    info!("Message loop ended, shutting down");
}