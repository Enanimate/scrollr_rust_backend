use std::{fs::File, io::Write, sync::OnceLock};
use log::{Level, Log};
use tokio::sync::mpsc;

pub use log::{info, error, warn, debug, trace};

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

pub async fn log_writer_task(mut receiver: mpsc::Receiver<LogMessage>, log_file_path: String) {
    let mut finance = match File::create(format!("{log_file_path}/finance.log")) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Fatal: Could not create log file at {:?}: {}", "finance.log", e);
            return;
        }
    };

    let mut sports = match File::create(format!("{log_file_path}/sports.log")) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Fatal: Could not create log file at {:?}: {}", "sports.log", e);
            return;
        }
    };

    println!("Starting async log writer task...");

    let mut sports_buffer = String::new();
    let mut finance_buffer = String::new();
    const LOG_BUFFER_FLUSH_SIZE: usize = 8192;


    while let Some(msg) = receiver.recv().await {
        println!("{msg}");

        if msg.contains("finance") {
            finance_buffer.push_str(&msg);
            finance_buffer.push('\n');
        }

        if msg.contains("sports") {
            sports_buffer.push_str(&msg);
            sports_buffer.push('\n');
        }

        if finance_buffer.len() > LOG_BUFFER_FLUSH_SIZE {
            if let Err(e) = finance.write_all(finance_buffer.as_bytes()) {
                eprintln!("Error writing log data to disk: {}", e);
            }
            finance_buffer.clear();
        }

        if sports_buffer.len() > LOG_BUFFER_FLUSH_SIZE {
            if let Err(e) = sports.write_all(sports_buffer.as_bytes()) {
                eprintln!("Error writing log data to disk: {}", e);
            }
            sports_buffer.clear();
        }
    }

    println!("Log writer task finished.");
}

const LOG_CHANNEL_CAPACITY: usize = 1000; 

pub fn init_async_logger(log_path: &str) -> Result<(), log::SetLoggerError> {
    let (sender, receiver) = mpsc::channel(LOG_CHANNEL_CAPACITY);

    let logger = AsyncLogger { sender };

    let res = log::set_logger(LOGGER.get_or_init(|| logger))
        .map(|()| log::set_max_level(log::LevelFilter::Info));

    if res.is_ok() {
        tokio::spawn(log_writer_task(receiver, log_path.to_owned()));
    }

    res
}