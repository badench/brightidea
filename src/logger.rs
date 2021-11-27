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

/// The Logger is a wrapper around a HashMap. This allows us to pass a reference to a Logger
/// as part of our application state through a warp filter. The Logger keeps a map of rooms to a
/// LogWriterHandle. The LogWriterHandle will be cloned and return to clients so that many clients
/// can send messages to be logged. The use of mpsc channels helps reduce backpressure experienced
/// by clients and allows th clients to work independently from how fast the logs are being written.
/// The underlying data structure of a HashMap was chosen to provide O(1) lookup for clients getting
/// a LogWriterHandle
pub struct Logger {
    log_map: LogMap,
}

impl Logger {
    /// Create a new Logger. The default state is an empty map.
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
    /// Create a new LogWriterHandle. This also creates the mpsc channel to interact with the
    /// LogWriter. A new LogWriter is created then a new tokio task is spawned which runs the
    /// LogWriter's run method, an infinite loop to write messages to a file.
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

    /// Public method for clients to interact with our LogWriterHandle. Clients send a message
    /// which this function takes ownership of and will send to the LogWriter to be logged.
    pub async fn log_message(&self, message: String) -> std::result::Result<(), SendError<String>> {
        self.tx.send(message)
    }
}

struct LogWriter {
    rx: UnboundedReceiverStream<String>,
    file: File,
}

/// A LogWriter represents the writer to a single log file. The Writer has a receiver end
/// of a mpsc channel. Clients interact through the use of the LogWriterHandle which holds
/// the sender side. The receiver end will listen and write logs to the
/// file opened at path = logs/<room_name>.log
impl LogWriter {
    /// Create a new LogWriter
    /// args:
    /// room_name: The chat room name which will act as the log file name
    /// rx: the receiver end of the create mpsc channel.
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

    /// Log a message to a file.
    /// args:
    /// message: The formatted message to log to the File referenced in self.file
    async fn log_message(&mut self, message: String) {
        match self.file.write_all(message.as_bytes()).await {
            Ok(_) => { /* Nothing to do here */ }
            Err(e) => {
                eprintln!("Could not log message {} with error {}", message, e);
            }
        }
    }

    /// Run the LogWriter. This sits in a loop waiting to receive messages to log
    async fn run(&mut self) {
        while let Some(message) = self.rx.next().await {
            self.log_message(message).await;
        }
    }
}

#[cfg(test)]
mod test {
    /// Testing module for all logger code.
    use super::*;
    use tokio::io::{AsyncBufReadExt, BufReader};

    #[tokio::test]
    async fn test_default_logger() {
        let logger = Logger::new();
        assert_eq!(0, logger.log_map.read().await.len());
    }

    #[tokio::test]
    async fn test_logger_one_room() {
        let logger = Logger::new();
        match logger.get_log_writer("test").await {
            Some(_) => {
                assert_eq!(1, logger.log_map.read().await.len());
            }
            None => {
                assert!(false, "Failed to create a LogWriterHandle")
            }
        }
    }

    #[tokio::test]
    async fn test_log_writer_handle_log_message() {
        let logger = Logger::new();
        match logger.get_log_writer("test").await {
            Some(writer) => {
                match writer.log_message(String::from("test message\n")).await {
                    Ok(_) => {
                        let path = "logs/test.log";
                        match tokio::fs::File::open(path).await {
                            Ok(file) => {
                                let mut buf_reader = BufReader::new(file);
                                let mut log_line = String::new();
                                let _num_bytes = buf_reader.read_line(&mut log_line).await;
                                assert_eq!(String::from("test message\n"), log_line);
                            }
                            Err(e) => {
                                assert!(false, "{}", format!("Could not open the file at path {} error {}", path, e));
                            }
                        }
                    }
                    Err(e) => {
                        assert!(false, "Failed to write to log_writer. error {}", e);
                    }
                }
            }
            None => {
                assert!(false, "failed to log message using LogWriterHandle")
            }
        }
    }
}