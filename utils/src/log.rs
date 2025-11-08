use std::{fs::File, io::Write, path::PathBuf, sync::OnceLock};
use log::{Level, Log};
use tokio::sync::mpsc;

pub use log::{info, error};

type LogMessage = String;

static LOGGER: OnceLock<AsyncLogger> = OnceLock::new();

pub struct AsyncLogger {
    sender: mpsc::Sender<LogMessage>,
}

impl Log for AsyncLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let log_entry = format!(
                "[{}] {} {} - {}",
                chrono::Local::now(),
                record.level(),
                record.target(),
                record.args()
            );

            let _ = self.sender.try_send(log_entry);
        }
    }

    fn flush(&self) {}
}

pub async fn log_writer_task(mut receiver: mpsc::Receiver<LogMessage>, log_file_path: PathBuf) {
    let mut file = match File::create(&log_file_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Fatal: Could not create log file at {:?}: {}", log_file_path, e);
            return;
        }
    };

    println!("Starting async log writer task...");

    let mut log_buffer = String::new();

    loop {
        match receiver.try_recv() {
            Ok(msg) => {
                log_buffer.push_str(&msg);
                log_buffer.push('\n');

                if log_buffer.len() > 8192 {
                    if let Err(e) = file.write_all(log_buffer.as_bytes()) {
                        eprintln!("Error writing log data to disk: {}", e);
                    }

                    log_buffer.clear();
                }
            }

            Err(mpsc::error::TryRecvError::Disconnected) => {
                break;
            }

            Err(mpsc::error::TryRecvError::Empty) => {
                if !log_buffer.is_empty() {
                    if let Err(e) = file.write_all(log_buffer.as_bytes()) {
                        eprintln!("Error writing remaining log data to disk: {}", e);
                    }
                    log_buffer.clear();
                }
                tokio::task::yield_now().await;
            }
        }
    }
}

const LOG_CHANNEL_CAPACITY: usize = 1000; 

pub fn init_async_logger(log_path: PathBuf) -> Result<(), log::SetLoggerError> {
    let (sender, receiver) = mpsc::channel(LOG_CHANNEL_CAPACITY);

    let logger = AsyncLogger { sender };

    let res = log::set_logger(LOGGER.get_or_init(|| logger))
        .map(|()| log::set_max_level(log::LevelFilter::Info));

    if res.is_ok() {
        tokio::spawn(log_writer_task(receiver, log_path));
    }

    res
}