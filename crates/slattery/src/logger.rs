// src/slattery/logger.rs

use std::sync::Mutex;
use std::collections::VecDeque;

#[derive(Clone)]
pub struct DevLogger {
    logs: std::sync::Arc<Mutex<VecDeque<String>>>,
    max_entries: usize,
}

impl DevLogger {
    pub fn new() -> Self {
        Self {
            logs: std::sync::Arc::new(Mutex::new(VecDeque::new())),
            max_entries: 1000,
        }
    }

    pub fn log(&self, message: &str, level: LogLevel) {
        let formatted = format!("[{}] {}", level.as_str(), message);
        let mut logs = self.logs.lock().unwrap();
        logs.push_back(formatted.clone());  // Clone here
        
        while logs.len() > self.max_entries {
            logs.pop_front();
        }
        
        match level {
            LogLevel::Error => eprintln!("[ERROR] {}", message),
            LogLevel::Warn => eprintln!("[WARN] {}", message),
            _ => println!("[{}] {}", level.as_str(), message),
        }
    }

    pub fn log_info(&self, msg: &str) {
        self.log(msg, LogLevel::Info);
    }

    pub fn log_error(&self, msg: &str) {
        self.log(msg, LogLevel::Error);
    }

    pub fn log_warn(&self, msg: &str) {
        self.log(msg, LogLevel::Warn);
    }

    pub fn log_debug(&self, msg: &str) {
        if std::env::var("SLATE_DEBUG").is_ok() {
            self.log(msg, LogLevel::Debug);
        }
    }

    pub fn get_logs(&self) -> Vec<String> {
        let logs = self.logs.lock().unwrap();
        logs.iter().cloned().collect()
    }

    pub fn clear(&self) {
        let mut logs = self.logs.lock().unwrap();
        logs.clear();
    }
}

#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
    Debug,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Debug => "DEBUG",
        }
    }
}