use agent_nvim::NvimSpecAgent;
use neovim_lib::{Neovim, Session};
use std::io::{self, BufRead, BufReader};
use std::process;
use serde_json::{Value, json};
use log::{info, error, debug};

fn main() {
    // Initialize logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    info!("Starting agent.nvim binary");

    // Create Neovim session
    let session = match Session::new_child() {
        Ok(session) => session,
        Err(e) => {
            error!("Failed to create Neovim session: {}", e);
            process::exit(1);
        }
    };

    // Initialize plugin
    let mut plugin = match NvimSpecAgent::new(session) {
        Ok(plugin) => plugin,
        Err(e) => {
            error!("Failed to initialize plugin: {}", e);
            process::exit(1);
        }
    };

    // Start the plugin
    if let Err(e) = plugin.start() {
        error!("Failed to start plugin: {}", e);
        process::exit(1);
    }

    info!("Plugin started successfully, entering message loop");

    // Message handling loop
    let stdin = io::stdin();
    let reader = BufReader::new(stdin.lock());

    for line in reader.lines() {
        match line {
            Ok(message) => {
                debug!("Received message: {}", message);
                
                if let Err(e) = handle_message(&mut plugin, &message) {
                    error!("Error handling message: {}", e);
                    // Send error response back to Neovim
                    let error_response = json!({
                        "type": "error",
                        "message": format!("Error: {}", e)
                    });
                    println!("{}", error_response);
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

fn handle_message(plugin: &mut NvimSpecAgent, message: &str) -> Result<(), Box<dyn std::error::Error>> {
    let parsed: Value = serde_json::from_str(message)?;
    
    let msg_type = parsed["type"].as_str().unwrap_or("unknown");
    
    match msg_type {
        "open_agent" => {
            plugin.open_agent()?;
            send_response("agent_opened", json!({"status": "success"}));
        }
        "close_agent" => {
            // Close agent interface
            send_response("agent_closed", json!({"status": "success"}));
        }
        "new_spec" => {
            let feature_name = parsed["data"]["feature_name"].as_str();
            plugin.new_spec(feature_name.map(|s| s.to_string()))?;
            send_response("spec_created", json!({
                "feature_name": feature_name.unwrap_or("new-feature")
            }));
        }
        "open_spec" => {
            let spec_name = parsed["data"]["spec_name"].as_str();
            plugin.open_spec(spec_name.map(|s| s.to_string()))?;
            send_response("spec_opened", json!({
                "spec_name": spec_name.unwrap_or("unknown")
            }));
        }
        "save_state" => {
            // Save plugin state
            send_response("state_saved", json!({"status": "success"}));
        }
        "handle_resize" => {
            // Handle window resize
            send_response("resize_handled", json!({"status": "success"}));
        }
        "ping" => {
            send_response("pong", json!({"status": "alive"}));
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