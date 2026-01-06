use clap::{Parser, Subcommand};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use super_whisper_linux::audio::AudioCapture;
use super_whisper_linux::config::{self, AppConfig};
use super_whisper_linux::ipc::{IpcClient, IpcServer};
use super_whisper_linux::{App, AppError};

#[derive(Parser)]
#[command(name = "super-whisper")]
#[command(about = "AI-powered voice to text for Linux")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Configuration file path
    #[arg(short, long)]
    config: Option<String>,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the application (default)
    Run,

    /// Send a command to the running instance
    #[command(subcommand)]
    Trigger(TriggerCommands),

    /// List available audio devices
    Devices,

    /// Show current status
    Status,

    /// Generate example configuration file
    InitConfig,
}

#[derive(Subcommand)]
enum TriggerCommands {
    /// Toggle recording
    Toggle,
    /// Start recording
    Start,
    /// Stop recording
    Stop,
    /// Cancel current operation
    Cancel,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let filter = if cli.debug {
        EnvFilter::new("super_whisper_linux=debug,info")
    } else {
        EnvFilter::new("super_whisper_linux=info,warn")
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    // Load configuration
    let config = config::load_config()?;

    match cli.command.unwrap_or(Commands::Run) {
        Commands::Run => run_app(config).await?,
        Commands::Trigger(cmd) => run_trigger(config, cmd).await?,
        Commands::Devices => list_devices()?,
        Commands::Status => show_status(config).await?,
        Commands::InitConfig => init_config()?,
    }

    Ok(())
}

async fn run_app(config: AppConfig) -> anyhow::Result<()> {
    info!("Starting SuperWhisper Linux");

    // Initialize directories
    config::init_dirs()?;

    // Create application
    let app = App::new(config.clone()).await?;

    // Initialize STT provider
    if let Err(e) = app.init_provider().await {
        error!("Failed to initialize STT provider: {}", e);
        error!("Hint: Make sure you have downloaded a whisper model to {:?}", config.model_path());
        return Err(e.into());
    }

    // Start IPC server
    let ipc_server = IpcServer::new(config.socket_path());
    let mut cmd_rx = ipc_server.start().await?;

    info!("Ready! Send commands via: echo 'toggle' | nc -U {:?}", config.socket_path());

    // Main event loop
    loop {
        tokio::select! {
            Some(cmd) = cmd_rx.recv() => {
                match app.handle_command(cmd).await {
                    Ok(_) => {}
                    Err(AppError::Other(msg)) if msg == "Shutdown" => {
                        info!("Shutting down");
                        break;
                    }
                    Err(e) => {
                        error!("Command error: {}", e);
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Received Ctrl+C, shutting down");
                break;
            }
        }
    }

    Ok(())
}

async fn run_trigger(config: AppConfig, cmd: TriggerCommands) -> anyhow::Result<()> {
    let client = IpcClient::new(config.socket_path());

    let command = match cmd {
        TriggerCommands::Toggle => "toggle",
        TriggerCommands::Start => "start",
        TriggerCommands::Stop => "stop",
        TriggerCommands::Cancel => "cancel",
    };

    match client.send(command).await {
        Ok(response) => {
            println!("{}", response);
        }
        Err(e) => {
            eprintln!("Error: {}. Is the app running?", e);
            std::process::exit(1);
        }
    }

    Ok(())
}

fn list_devices() -> anyhow::Result<()> {
    println!("Available audio input devices:");
    for device in AudioCapture::list_devices()? {
        println!("  - {}", device);
    }
    Ok(())
}

async fn show_status(config: AppConfig) -> anyhow::Result<()> {
    let client = IpcClient::new(config.socket_path());

    match client.send("status").await {
        Ok(response) => {
            println!("Status: {}", response);
        }
        Err(e) => {
            eprintln!("Error: {}. Is the app running?", e);
            std::process::exit(1);
        }
    }

    Ok(())
}

fn init_config() -> anyhow::Result<()> {
    let config_path = config::config_dir().join("config.toml");

    if config_path.exists() {
        eprintln!("Configuration file already exists at {:?}", config_path);
        std::process::exit(1);
    }

    config::init_dirs()?;
    config::save_config(&AppConfig::default())?;

    println!("Configuration file created at {:?}", config_path);
    println!("\nNext steps:");
    println!("1. Download a whisper model (multilingual for Spanish/English):");
    println!("   curl -L https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin -o ~/.local/share/super-whisper-linux/models/ggml-base.bin");
    println!("\n2. Add a hotkey to your Hyprland config:");
    println!("   bind = SUPER, V, exec, super-whisper trigger toggle");

    Ok(())
}
