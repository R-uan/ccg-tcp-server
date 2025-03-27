use chrono::Local;

pub struct Logger;

impl Logger {
    pub fn info(message: &str) {
        let local = Local::now().format("%d/%m/%Y %H:%M:%S");
        println!("[INFO] [{local}] {message}");
    }

    pub fn debug(message: &str) {
        let local = Local::now().format("%d/%m/%Y %H:%M:%S");
        println!("[DEBUG] [{local}] {message}");
    }

    pub fn warn(message: &str) {
        let local = Local::now().format("%d/%m/%Y %H:%M:%S");
        eprintln!("[WARN] [{local}] {message}");
    }

    pub fn error(message: &str) {
        let local = Local::now().format("%d/%m/%Y %H:%M:%S");
        eprintln!("[ERROR] [{local}] {message}");
    }
}
