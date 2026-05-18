mod collector;
mod config;
mod engine;
mod service;
mod util;

use std::env;
use anyhow::Result;
use crate::config::AgentConfig;

fn main() -> Result<()> {
    // Initialize logger
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    let command = args[1].as_str();
    match command {
        "install" => install_service()?,
        "uninstall" => uninstall_service()?,
        "start" => start_service()?,
        "stop" => stop_service()?,
        "status" => status_service()?,
        "run" => run_foreground()?,
        _ => {
            println!("Unknown command: {}", command);
            print_usage();
        }
    }

    Ok(())
}

fn print_usage() {
    println!("Usage: agent.exe <command>");
    println!("Commands:");
    println!("  install   - Register as a Windows service");
    println!("  uninstall - Unregister the Windows service");
    println!("  start     - Start the Windows service");
    println!("  stop      - Stop the Windows service");
    println!("  status    - Check service status");
    println!("  run       - Run in foreground (debug mode)");
}

fn install_service() -> Result<()> {
    println!("Installing service...");
    // TODO: Implement service installation logic using windows-service or sc.exe
    println!("Service installed successfully (mock)");
    Ok(())
}

fn uninstall_service() -> Result<()> {
    println!("Uninstalling service...");
    // TODO: Implement service uninstallation logic
    println!("Service uninstalled successfully (mock)");
    Ok(())
}

fn start_service() -> Result<()> {
    println!("Starting service...");
    // TODO: Implement service start logic
    println!("Service started (mock)");
    Ok(())
}

fn stop_service() -> Result<()> {
    println!("Stopping service...");
    // TODO: Implement service stop logic
    println!("Service stopped (mock)");
    Ok(())
}

fn status_service() -> Result<()> {
    println!("Checking service status...");
    // TODO: Implement service status check logic
    println!("Service status: Running (mock)");
    Ok(())
}

fn run_foreground() -> Result<()> {
    println!("Running in foreground...");
    let config = AgentConfig::load()?;
    println!("Config loaded: {:?}", config);
    
    // In foreground mode, we don't need the service dispatcher
    // We can just run the main loop directly
    loop {
        // TODO: Implement collection and reporting logic
        println!("Agent is running...");
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
}
