use std::io::{self, BufRead, BufReader};
use serde_json::{Value, json};
use log::{info, error, debug};

fn main() {
    // Initialize logging to stderr so it doesn't interfere with JSON communication
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .target(env_logger::Target::Stderr)
        .init();

    info!("Starting agent.nvim binary");

    // Send startup confirmation
    send_response("startup", json!({"status": "ready"}));

    // Message handling loop - no Neovim session needed for basic communication
    let stdin = io::stdin();
    let reader = BufReader::new(stdin.lock());

    for line in reader.lines() {
        match line {
            Ok(message) => {
                debug!("Received message: {}", message);
                
                if let Err(e) = handle_message(&message) {
                    error!("Error handling message: {}", e);
                    send_response("error", json!({
                        "message": format!("Error: {}", e)
                    }));
                }
            }
            Err(e) => {
                error!("Error reading from stdin: {}", e);
                break;
            }
        }
    }

    info!("Message loop ended, shutting down");
}

fn handle_message(message: &str) -> Result<(), Box<dyn std::error::Error>> {
    let parsed: Value = serde_json::from_str(message)?;
    
    let msg_type = parsed["type"].as_str().unwrap_or("unknown");
    
    match msg_type {
        "ping" => {
            send_response("pong", json!({"status": "alive"}));
        }
        "open_agent" => {
            send_response("agent_opened", json!({"status": "success"}));
        }
        "chat_message" => {
            let message = parsed["data"]["message"].as_str().unwrap_or("");
            info!("Received chat message: {}", message);
            
            // Echo back a simple response for now
            send_response("chat_response", json!({
                "message": format!("**Agent:** I received your message: \"{}\"", message)
            }));
        }
        "close_agent" => {
            send_response("agent_closed", json!({"status": "success"}));
        }
        "new_spec" => {
            let feature_name = parsed["data"]["feature_name"].as_str().unwrap_or("new-feature");
            send_response("spec_created", json!({
                "feature_name": feature_name
            }));
        }
        "open_spec" => {
            let spec_name = parsed["data"]["spec_name"].as_str().unwrap_or("unknown");
            send_response("spec_opened", json!({
                "spec_name": spec_name
            }));
        }
        "save_state" => {
            send_response("state_saved", json!({"status": "success"}));
        }
        "handle_resize" => {
            send_response("resize_handled", json!({"status": "success"}));
        }
        _ => {
            error!("Unknown message type: {}", msg_type);
            send_response("error", json!({
                "message": format!("Unknown message type: {}", msg_type)
            }));
        }
    }
    
    Ok(())
}

fn send_response(response_type: &str, data: Value) {
    let response = json!({
        "type": response_type,
        "data": data
    });
    println!("{}", response);
}