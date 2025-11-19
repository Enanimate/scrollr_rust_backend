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
                "[{}] {} {} - {}\n",
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

    let mut fantasy = match File::create(format!("{log_file_path}/fantasy.log")) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Fatal: Could not create log file at {:?}: {}", "fantasy.log", e);
            return;
        }
    };

    let mut backend = match File::create(format!("{log_file_path}/backend.log")) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Fatal: Could not create log file at {:?}: {}", "backend.log", e);
            return;
        }
    };

    println!("Starting async log writer task...");

    while let Some(msg) = receiver.recv().await {
        println!("{msg}");

        if msg.contains("finance") {
            if let Err(e) = finance.write_all(msg.as_bytes()) {
                eprintln!("Error writing log data to disk: {}", e);
            }
        }

        if msg.contains("sports") {
            if let Err(e) = sports.write_all(msg.as_bytes()) {
                eprintln!("Error writing log data to disk: {}", e);
            }
        }

        if msg.contains("fantasy") {
            if let Err(e) = fantasy.write_all(msg.as_bytes()) {
                eprintln!("Error writing log data to disk: {}", e);
            }
        }

        if msg.contains("backend") {
            if let Err(e) = backend.write_all(msg.as_bytes()) {
                eprintln!("Error writing log data to disk: {}", e);
            }
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