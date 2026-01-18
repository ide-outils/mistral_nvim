use std::{
    sync::{LazyLock, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

use tokio::sync::mpsc::{self, Receiver, Sender};

use super::notify;

pub type Level = notify::NotifyLevel;

type LibuvSender<T> = Sender<T>;
type TokioSender<T> = Sender<T>;

static LOG_SENDERS: LazyLock<(LibuvSender<Log>, TokioSender<Log>)> = LazyLock::new(|| {
    let (tx_libuv, rx_libuv) = mpsc::channel::<Log>(10000);
    let (tx_tokio, rx_tokio) = mpsc::channel::<Log>(100);
    start_logger_task(rx_libuv, rx_tokio);
    (tx_libuv, tx_tokio)
});

/// Can be modified with nvim command `:MistralLogLevel 2` or `:MistralLogLevel Info`
#[cfg(feature = "prod_mode")]
#[cfg(feature = "log_trace")]
pub static THRESHOLD: LazyLock<RwLock<Level>> = LazyLock::new(|| Level::Trace.into());
#[cfg(feature = "prod_mode")]
#[cfg(not(feature = "log_trace"))]
pub static THRESHOLD: LazyLock<RwLock<Level>> = LazyLock::new(|| Level::Warn.into());
#[cfg(not(feature = "prod_mode"))]
pub static THRESHOLD: LazyLock<RwLock<Level>> = LazyLock::new(|| Level::Off.into());

#[cfg(not(feature = "no_logs"))]
impl Level {
    pub fn set_by_args(&mut self, args: nvim_oxi::api::types::CommandArgs) {
        let Some(arg) = args.fargs.get(0) else { return };
        use notify::NotifyLevel::*;
        if let Ok(number) = arg.parse::<usize>() {
            *self = match number {
                0 => Trace,
                1 => Debug,
                2 => Info,
                3 => Warn,
                4 => Error,
                5 => Off,
                _ => {
                    super::notify::warn("Maximum level is 5. Level set to Warn (3).");
                    Warn
                }
            };
        } else {
            *self = match arg.as_str() {
                "Trace" => Trace,
                "Debug" => Debug,
                "Info" => Info,
                "Warn" => Warn,
                "Error" => Error,
                "Off" => Off,
                _ => {
                    super::notify::warn("Level does not exist. Level set to Warn (3).");
                    Warn
                }
            };
        }
    }
}

pub struct Log {
    level: Level,
    message: String,
}

impl Log {
    pub fn new(level: Level, message: String) -> Self {
        Self { level, message }
    }
}

#[macro_export]
macro_rules! log_libuv {
    ($level:ident, $($arg:tt)*) => {{
        #[cfg(feature = "prod_mode")]
        #[cfg(not(feature = "no_logs"))]
        {
            // let message = format!("{}", format_args!($($arg)*));
            let message = format!("`{}` {}", std::panic::Location::caller(), format_args!($($arg)*));
            use crate::utils::logger::{Level, Log, send_logs_libuv};
            send_logs_libuv(Log::new(Level::$level, message))
        }
        #[cfg(not(feature = "prod_mode"))]
        {
            // let message = format!("{}", format_args!($($arg)*));
            let message = format!("`{}` {}", std::panic::Location::caller(), format_args!($($arg)*));
            use crate::utils::logger::{Level, Log, format_log};
            let log = Log::new(Level::$level, message);
            if let Some(msg) = format_log("libuv", log) {
                println!("LOGGER PRINT {msg}");
            }
        }
    }};
}
#[macro_export]
macro_rules! log_tokio {
    ($level:ident, $($arg:tt)*) => {{
        #[cfg(feature = "prod_mode")]
        #[cfg(not(feature = "no_logs"))]
        {
            // let message = format!("{}", format_args!($($arg)*));
            let message = format!("`{}` {}", std::panic::Location::caller(), format_args!($($arg)*));
            use crate::utils::logger::{Level, Log, send_logs_tokio};
            send_logs_tokio(Log::new(Level::$level, message))
        }
        #[cfg(not(feature = "prod_mode"))]
        {
            // let message = format!("{}", format_args!($($arg)*));
            let message = format!("`{}` {}", std::panic::Location::caller(), format_args!($($arg)*));
            use crate::utils::logger::{Level, Log, format_log};
            let log = Log::new(Level::$level, message);
            if let Some(msg) = format_log("libuv", log) {
                println!("LOGGER PRINT {msg}");
            }
        }
    }};
}
pub fn send_logs_libuv(log: Log) {
    send_log(&LOG_SENDERS.0, log, "libuv", 10);
}
pub fn send_logs_tokio(log: Log) {
    send_log(&LOG_SENDERS.1, log, "tokio", 5);
}
#[inline]
fn send_log(sender: &Sender<Log>, log: Log, queue_name: &'static str, max_attemps: usize) {
    let mut prev_try = log;
    let mut attemps = 0;
    loop {
        match sender.try_send(prev_try) {
            Ok(_) => break,
            Err(error) => match error {
                mpsc::error::TrySendError::Full(log) => {
                    attemps += 1;
                    prev_try = log;
                    if attemps > max_attemps {
                        eprintln!("Error : {queue_name} log is full for too long.");
                        println!("Error : {queue_name} log is full for too long.");
                        break;
                    }
                }
                mpsc::error::TrySendError::Closed(_) => {
                    eprintln!("Error : {queue_name} log channel has been closed.");
                    println!("Error : {queue_name} log channel has been closed.");
                    break;
                }
            },
        }
    }
}

#[cfg(not(feature = "prod_mode"))]
fn start_logger_task(_libuv_rx: Receiver<Log>, _tokio_rx: Receiver<Log>) {}

#[cfg(feature = "prod_mode")]
#[track_caller]
fn start_logger_task(mut libuv_rx: Receiver<Log>, mut tokio_rx: Receiver<Log>) {
    tokio::spawn(async move {
        loop {
            tokio::select! {
                msg = libuv_rx.recv() => {
                    if let Some(msg) = msg {
                        if let Some(msg) = format_log("libuv", msg) {
                            write_formatted_log(msg).await;
                        }
                    } else {
                        break;
                    }
                },
                msg = tokio_rx.recv() => {
                    if let Some(msg) = msg {
                        if let Some(msg) = format_log("tokio", msg) {
                            write_formatted_log(msg).await;
                        }
                    } else {
                        break;
                    }
                },
            }
        }
    });
}

pub fn format_log(logger: &str, log: Log) -> Option<String> {
    if log.level >= *THRESHOLD.read().unwrap() {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let message = format!("[{}:{}:{}] {}", logger, timestamp, log.level, log.message);
        Some(message)
        // write_formatted_log(message).await;
    } else {
        None
    }
}

#[cfg(not(test))]
async fn write_formatted_log(message: String) {
    use tokio::{fs::OpenOptions, io::AsyncWriteExt};
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("nvim_mistral.log")
        .await
        .unwrap();
    file.write_all(message.as_bytes()).await.unwrap();
    file.write_all(b"\n").await.unwrap();
    file.flush().await.unwrap();
}
