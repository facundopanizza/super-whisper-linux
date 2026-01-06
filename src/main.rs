use clap::{Parser, Subcommand};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use super_whisper_linux::audio::AudioCapture;
use super_whisper_linux::config::{self, AppConfig};
use super_whisper_linux::ipc::{IpcClient, IpcServer};
use super_whisper_linux::tray::TrayIcon;
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

    /// Download a whisper model
    DownloadModel {
        /// Model variant: tiny, base, small, medium, large
        #[arg(short, long, default_value = "base")]
        model: String,
    },
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
        Commands::DownloadModel { model } => download_model(&model).await?,
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
    let socket_path = config.socket_path();
    let ipc_server = IpcServer::new(socket_path.clone());
    let mut cmd_rx = ipc_server.start().await?;

    // Initialize system tray (keep _tray alive to maintain the tray service)
    let _tray = TrayIcon::new(socket_path.to_string_lossy().to_string())?;
    let tray_handle = _tray.handle();

    // Spawn task to sync app state with tray
    let mut state_rx = app.state_receiver();
    tokio::spawn(async move {
        loop {
            if state_rx.changed().await.is_err() {
                break;
            }
            let state = *state_rx.borrow();
            tray_handle.set_state(state.to_tray_state());
        }
    });

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
    println!("1. Download a whisper model:");
    println!("   super-whisper-linux download-model --model base");
    println!("\n2. Add a hotkey to your Hyprland config:");
    println!("   bind = SUPER, B, exec, super-whisper-linux trigger toggle");

    Ok(())
}

async fn download_model(model: &str) -> anyhow::Result<()> {
    use futures::StreamExt;
    use std::io::Write;

    // Validate model name
    let valid_models = ["tiny", "base", "small", "medium", "large"];
    if !valid_models.contains(&model) {
        eprintln!("Invalid model: {}. Valid options: {}", model, valid_models.join(", "));
        std::process::exit(1);
    }

    let filename = format!("ggml-{}.bin", model);
    let url = format!(
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{}",
        filename
    );

    // Create models directory
    config::init_dirs()?;
    let models_dir = config::data_dir().join("models");
    std::fs::create_dir_all(&models_dir)?;

    let dest_path = models_dir.join(&filename);

    if dest_path.exists() {
        println!("Model already exists at {:?}", dest_path);
        println!("Delete it first if you want to re-download.");
        return Ok(());
    }

    println!("Downloading {} model from HuggingFace...", model);
    println!("URL: {}", url);
    println!("Destination: {:?}", dest_path);
    println!();

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .send()
        .await?
        .error_for_status()?;

    let total_size = response.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;

    let mut file = std::fs::File::create(&dest_path)?;
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk)?;
        downloaded += chunk.len() as u64;

        if total_size > 0 {
            let percent = (downloaded as f64 / total_size as f64) * 100.0;
            print!("\rDownloading: {:.1}% ({:.1} MB / {:.1} MB)",
                percent,
                downloaded as f64 / 1_000_000.0,
                total_size as f64 / 1_000_000.0
            );
            std::io::stdout().flush()?;
        }
    }

    println!("\n\nDownload complete!");
    println!("Model saved to: {:?}", dest_path);

    Ok(())
}
