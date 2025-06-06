use std::fmt::Arguments;
use chrono::Local;

pub struct Logger;

impl Logger {
    pub fn info(args: Arguments) {
        let local = Local::now().format("%d/%m/%Y %H:%M:%S");
        println!("[INFO ] [{local}] {args}");
    }

    pub fn debug(args: Arguments) {
        let local = Local::now().format("%d/%m/%Y %H:%M:%S");
        println!("[DEBUG] [{local}] {args}");
    }

    pub fn warn(args: Arguments) {
        let local = Local::now().format("%d/%m/%Y %H:%M:%S");
        eprintln!("[WARN ] [{local}] {args}");
    }

    pub fn error(args: Arguments) {
        let local = Local::now().format("%d/%m/%Y %H:%M:%S");
        eprintln!("[ERROR] [{local}] {args}");
    }
}

#[macro_export]
macro_rules! logger {
    (INFO, $($arg:tt)*) => {
        Logger::info(format_args!($($arg)*))
    };
    (DEBUG, $($arg:tt)*) => {
        Logger::debug(format_args!($($arg)*))
    };
    (WARN, $($arg:tt)*) => {
        Logger::warn(format_args!($($arg)*))
    };
    (ERROR, $($arg:tt)*) => {
        Logger::error(format_args!($($arg)*))
    };
}