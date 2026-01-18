use crate::notify::{Notification, NotifyLevel, debug, error, info, off, trace, warn};

pub mod controlleur;
pub mod model;
pub mod vue;

impl NotifyLevel {
    pub(super) fn notify(&self, msg: &str) {
        let notify_fn = match self {
            NotifyLevel::Trace => trace,
            NotifyLevel::Debug => debug,
            NotifyLevel::Info => info,
            NotifyLevel::Warn => warn,
            NotifyLevel::Error => error,
            NotifyLevel::Off => off,
        };
        notify_fn(msg);
    }
}

impl Notification {
    pub(super) fn notify(&self) {
        self.level.notify(&self.message)
    }
}
