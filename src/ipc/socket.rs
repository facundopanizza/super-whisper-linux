use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::error::IpcError;

/// Commands that can be sent via IPC
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpcCommand {
    /// Toggle recording on/off
    Toggle,
    /// Start recording
    Start,
    /// Stop recording and transcribe
    Stop,
    /// Cancel current operation
    Cancel,
    /// Get current status
    Status,
    /// Shutdown the application
    Shutdown,
}

impl IpcCommand {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "toggle" => Some(Self::Toggle),
            "start" => Some(Self::Start),
            "stop" => Some(Self::Stop),
            "cancel" => Some(Self::Cancel),
            "status" => Some(Self::Status),
            "shutdown" | "quit" | "exit" => Some(Self::Shutdown),
            _ => None,
        }
    }
}

/// IPC server that listens for commands
pub struct IpcServer {
    socket_path: PathBuf,
}

impl IpcServer {
    pub fn new(socket_path: PathBuf) -> Self {
        Self { socket_path }
    }

    /// Start the IPC server and return a receiver for commands
    pub async fn start(&self) -> Result<mpsc::Receiver<IpcCommand>, IpcError> {
        // Clean up old socket
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path)?;
        }

        let listener = UnixListener::bind(&self.socket_path)?;

        // Set permissions (user only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&self.socket_path, std::fs::Permissions::from_mode(0o600))?;
        }

        info!("IPC server listening on {:?}", self.socket_path);

        let (tx, rx) = mpsc::channel::<IpcCommand>(32);

        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, _)) => {
                        let tx = tx.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_client(stream, tx).await {
                                warn!("IPC client error: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        error!("IPC accept error: {}", e);
                    }
                }
            }
        });

        Ok(rx)
    }
}

impl Drop for IpcServer {
    fn drop(&mut self) {
        // Clean up socket file
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

async fn handle_client(
    mut stream: UnixStream,
    tx: mpsc::Sender<IpcCommand>,
) -> Result<(), IpcError> {
    let (reader, mut writer) = stream.split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    reader.read_line(&mut line).await?;
    debug!("IPC received: {}", line.trim());

    let response = if let Some(cmd) = IpcCommand::from_str(&line) {
        match tx.send(cmd).await {
            Ok(_) => "OK\n",
            Err(_) => "ERROR: Channel closed\n",
        }
    } else {
        "ERROR: Unknown command\n"
    };

    writer.write_all(response.as_bytes()).await?;
    Ok(())
}

/// IPC client for sending commands
pub struct IpcClient {
    socket_path: PathBuf,
}

impl IpcClient {
    pub fn new(socket_path: PathBuf) -> Self {
        Self { socket_path }
    }

    /// Send a command to the server
    pub async fn send(&self, command: &str) -> Result<String, IpcError> {
        let mut stream = UnixStream::connect(&self.socket_path)
            .await
            .map_err(|_| IpcError::ConnectionRefused)?;

        stream.write_all(command.as_bytes()).await?;
        stream.write_all(b"\n").await?;

        let (reader, _) = stream.split();
        let mut reader = BufReader::new(reader);
        let mut response = String::new();
        reader.read_line(&mut response).await?;

        Ok(response.trim().to_string())
    }
}
