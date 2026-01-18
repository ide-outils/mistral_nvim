use nvim_oxi::{
    Dictionary,
    api::{notify, types::LogLevel},
};
use serde::Serialize;

pub trait NotifyExt {
    fn notify_error(&self);
    fn notify_warn(&self);
}
impl<T, E: std::fmt::Display> NotifyExt for std::result::Result<T, E> {
    #[track_caller]
    fn notify_error(&self) {
        if let Err(err) = self {
            error(err);
        }
    }
    #[track_caller]
    fn notify_warn(&self) {
        if let Err(err) = self {
            warn(err);
        }
    }
}
pub trait NotifyExtV2 {
    fn notify(&self);
}
impl<T> NotifyExtV2 for crate::Result<T> {
    #[track_caller]
    fn notify(&self) {
        if let Err(notification) = self {
            notification.notify();
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum NotifyLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
    Off = 5,
}
impl Into<LogLevel> for NotifyLevel {
    fn into(self) -> LogLevel {
        use NotifyLevel::*;
        match self {
            Trace => LogLevel::Trace,
            Debug => LogLevel::Debug,
            Info => LogLevel::Info,
            Warn => LogLevel::Warn,
            Error => LogLevel::Error,
            Off => LogLevel::Off,
        }
    }
}

#[cfg(feature = "prod_mode")]
pub fn trace(message: impl ToString) {
    let msg = message.to_string();
    crate::log_libuv!(Trace, "{msg}");
}
#[cfg(not(feature = "prod_mode"))]
pub fn trace(message: impl ToString) {
    let msg = message.to_string();
    crate::log_libuv!(Trace, "{msg}");
    notify(&msg, LogLevel::Trace, &Dictionary::new()).expect("Can not notify.");
}
#[cfg(feature = "prod_mode")]
pub fn debug(message: impl ToString) {
    let msg = message.to_string();
    crate::log_libuv!(Debug, "{msg}");
}
#[cfg(not(feature = "prod_mode"))]
pub fn debug(message: impl ToString) {
    let msg = message.to_string();
    crate::log_libuv!(Debug, "{msg}");
    notify(&msg, LogLevel::Debug, &Dictionary::new()).expect("Can not notify.");
}
pub fn info(message: impl ToString) {
    let msg = message.to_string();
    crate::log_libuv!(Info, "{msg}");
    notify(&msg, LogLevel::Info, &Dictionary::new()).expect("Can not notify.");
}
#[cfg(test)]
pub fn warn(message: impl ToString) {
    let msg = message.to_string();
    crate::log_libuv!(Warn, "{msg}");
}
#[cfg(not(test))]
pub fn warn(message: impl ToString) {
    let msg = message.to_string();
    crate::log_libuv!(Warn, "{msg}");
    notify(&msg, LogLevel::Warn, &Dictionary::new()).expect("Can not notify.");
}
#[cfg(test)]
pub fn error(message: impl ToString) {
    let msg = message.to_string();
    crate::log_libuv!(Error, "{msg}");
}
#[cfg(not(test))]
#[track_caller]
pub fn error(message: impl ToString) {
    let msg = format!("[{}] : {}", std::panic::Location::caller(), message.to_string());
    crate::log_libuv!(Error, "{msg}");
    notify(&msg, LogLevel::Error, &Dictionary::new()).expect("Can not notify.");
}
pub fn off(message: impl ToString) {
    let msg = message.to_string();
    crate::log_libuv!(Off, "{msg}");
    notify(&msg, LogLevel::Off, &Dictionary::new()).expect("Can not notify.");
}

#[derive(Debug, Serialize)]
pub struct Notification {
    pub level: NotifyLevel,
    pub message: String,
    pub location: String,
}
impl std::error::Error for Notification {}

impl Notification {
    #[track_caller]
    pub fn new(level: NotifyLevel, message: impl ToString) -> Self {
        Self {
            level,
            message: message.to_string(),
            location: std::panic::Location::caller().to_string(),
        }
    }
    // pub fn notify(self) {
    //     notify(&self.message, self.level.into(), &Dictionary::new()).expect("Can not notify.");
    // }
}

use NotifyLevel::*;
pub trait IntoNotification: std::fmt::Display + Sized {
    #[track_caller]
    fn into_off(self) -> Notification {
        Notification::new(Off, self)
    }
    #[track_caller]
    fn into_error(self) -> Notification {
        Notification::new(Error, self)
    }
    #[track_caller]
    fn into_warn(self) -> Notification {
        Notification::new(Warn, self)
    }
    #[track_caller]
    fn into_info(self) -> Notification {
        Notification::new(Info, self)
    }
    #[track_caller]
    fn into_debug(self) -> Notification {
        Notification::new(Debug, self)
    }
    #[track_caller]
    fn into_trace(self) -> Notification {
        Notification::new(Trace, self)
    }
}

impl std::fmt::Display for Notification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {} : {}", self.location, self.level, self.message)
    }
}
impl std::fmt::Display for NotifyLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let repr = match self {
            Trace => "Trace",
            Debug => "Debug",
            Info => "Info",
            Warn => "Warn",
            Error => "Error",
            Off => "Off",
        };
        write!(f, "{repr}")
    }
}

pub type Result<T> = std::result::Result<T, Notification>;

// // Builtins
// impl IntoNotification for String {}
// impl IntoNotification for &str {}
// impl IntoNotification for std::io::Error {}
// impl<T> IntoNotification for std::sync::PoisonError<T> {}

// // nvim_oxi
// impl IntoNotification for nvim_oxi::api::Error {}
// impl IntoNotification for nvim_oxi::Error {}
// // serde_json
// impl IntoNotification for serde_json::Error {}
// // tree_sitter
// impl IntoNotification for tree_sitter::QueryError {}
// impl IntoNotification for tree_sitter::LanguageError {}

macro_rules! impl_into_notif_for {
    ($error_type:ty) => {
        impl From<$error_type> for Notification {
            #[track_caller]
            fn from(value: $error_type) -> Self {
                value.into_error()
            }
        }
        impl From<(NotifyLevel, $error_type)> for Notification {
            #[track_caller]
            fn from(value: (NotifyLevel, $error_type)) -> Self {
                Notification {
                    level: value.0,
                    message: value.1.to_string(),
                    location: std::panic::Location::caller().to_string(),
                }
            }
        }
        impl IntoNotification for $error_type {}
    };
}

// // Builtins
// impl IntoNotification for String {}
// impl IntoNotification for &str {}
// impl IntoNotification for std::io::Error {}
// impl<T> IntoNotification for std::sync::PoisonError<T> {}

// // nvim_oxi
// impl IntoNotification for nvim_oxi::api::Error {}
// impl IntoNotification for nvim_oxi::Error {}
// // serde_json
// impl IntoNotification for serde_json::Error {}
// // tree_sitter
// impl IntoNotification for tree_sitter::QueryError {}
// impl IntoNotification for tree_sitter::LanguageError {}

// // Builtins
impl_into_notif_for!(String);
impl_into_notif_for!(&str);
impl_into_notif_for!(std::io::Error);
// impl_into_notif_for!(std::sync::PoisonError<std::sync::MutexGuard<>>>);

// nvim_oxi
impl_into_notif_for!(nvim_oxi::api::Error);
impl_into_notif_for!(nvim_oxi::Error);
// serde_json
impl_into_notif_for!(serde_json::Error);
// tree_sitter
impl_into_notif_for!(tree_sitter::QueryError);
impl_into_notif_for!(tree_sitter::LanguageError);

impl<T> From<std::sync::PoisonError<T>> for Notification {
    #[track_caller]
    fn from(value: std::sync::PoisonError<T>) -> Self {
        Notification {
            level: NotifyLevel::Error,
            message: value.to_string(),
            location: std::panic::Location::caller().to_string(),
        }
    }
}
#[cfg(not(feature = "prod_mode"))]
impl From<(NotifyLevel, Notification)> for Notification {
    #[track_caller]
    fn from(value: (NotifyLevel, Self)) -> Self {
        value.1
    }
}
