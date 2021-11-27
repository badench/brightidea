use tokio::sync::{mpsc, RwLock};
use std::collections::HashMap;
use std::io::Result;
use std::sync::{Arc};
use futures_util::StreamExt;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc::error::SendError;

type LogMap = Arc<RwLock<HashMap<String, LogWriterHandle>>>;

/// The Logger holds all LogWriterHandles in a map for different log files. LogWriterHandles
/// can be cloned and returned to clients to facilitate multiple clients sending messages to logged.
pub struct Logger {
    log_map: LogMap,
}

impl Logger {
    pub fn new() -> Logger {
        Logger {
            log_map: LogMap::default(),
        }
    }
    /// Returns a LogWriterHandle the client can use to send log messages
    pub async fn get_log_writer(&self, room_name: &str) -> Option<LogWriterHandle> {
        let mut write_lock = self.log_map.write().await;
        match write_lock.get(room_name) {
            Some(writer) => {
                // If LogWriterHandle exists, return a cloned copy
                Some(writer.clone())
            }
            None => {
                // If it does not exist create one and put it in map. Return a clone
                match LogWriterHandle::new(room_name).await {
                    Some(log_writer) => {
                        let writer = log_writer.clone();
                        write_lock.insert(room_name.to_string(), log_writer);
                        Some(writer)
                    }
                    None => {
                        // Some error occurred creating the handle
                        None
                    }
                }
            }
        }
    }
}

/// The LogWriterHandle represents the sending side of an mpsc channel. The receiver is a LogWriter
/// whose purpose is to log messages. The Handle is necessary to implement the Actor pattern on top
/// of the tokio runtime. The LogWriterHandle creates the channel and spawns a new LogWriter task
/// which loops forever waiting for messages.
#[derive(Clone)]
pub struct LogWriterHandle {
    tx: mpsc::UnboundedSender<String>,
}

impl LogWriterHandle {
    async fn new(room_name: &str) -> Option<Self> {
        let (tx, rx) = mpsc::unbounded_channel();
        let rx = UnboundedReceiverStream::new(rx);
        match LogWriter::new(room_name, rx).await {
            Ok(mut log_writer) => {
                tokio::task::spawn(async move { log_writer.run().await; });
                Some(Self { tx })
            }
            Err(e) => {
                eprintln!("Error creating LogWriter {}", e);
                None
            }
        }
    }
    pub async fn log_message(&self, message: String) -> std::result::Result<(), SendError<String>> {
        self.tx.send(message)
    }
}

struct LogWriter {
    rx: UnboundedReceiverStream<String>,
    file: File,
}

impl LogWriter {
    /// Create a new LogWriter. A LogWriter represents the writer to a single log file.
    /// The Writer has a receiver end of a mpsc channel. Clients interact through the use
    /// of the LogWriterHandle which holds the sender side. The receiver end will listen and write logs
    /// to the file opened at path = logs/<room_name>.log
    pub async fn new(room_name: &str, rx: UnboundedReceiverStream<String>) -> Result<Self> {
        let path = format!("logs/{}.log", room_name);
        let file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)
            .await;
        match file {
            Ok(file) => {
                Ok(LogWriter {
                    rx,
                    file,
                })
            }
            Err(e) => {
                Err(e)
            }
        }
    }

    async fn log_message(&mut self, message: String) {
        match self.file.write_all(message.as_bytes()).await {
            Ok(_) => { /* Nothing to do here */ }
            Err(e) => {
                eprintln!("Could not log message {} with error {}", message, e);
            }
        }
    }

    async fn run(&mut self) {
        while let Some(message) = self.rx.next().await {
            self.log_message(message).await;
        }
    }
}