use chrono::Utc;

use std::{fs::OpenOptions, io::Write};

#[derive(Debug)]
pub struct Logger {
    filepath: String,
}

impl Logger {
    pub fn new(filepath: String) -> Logger {
        if let Err(e) = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&filepath)
        {
            println!("ERROR OPENING LOGFILE: {}", e);
        };

        Logger { filepath }
    }

    pub fn log(&self, message: String) {
        let res = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.filepath);

        let mut file = match res {
            Ok(f) => f,
            Err(e) => {
                eprintln!("LOGGING ERROR: Couldn't open the log file: {}", e);
                return;
            }
        };

        let msg = format!("{}: {}", Utc::now().format("%y-%m-%d %H:%M:%S"), message);
        if let Err(e) = writeln!(file, "{}", msg) {
            eprintln!("LOGGING ERROR: Couldn't write to file: {}", e);
        }
    }

    pub fn log_error(&self, error: String) {
        let res = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.filepath);

        let mut file = match res {
            Ok(f) => f,
            Err(e) => {
                eprintln!("LOGGING ERROR: Couldn't open the log file: {}", e);
                return;
            }
        };

        let msg = format!("ERROR: {}", error);
        if let Err(e) = writeln!(file, "{}", msg) {
            eprintln!("LOGGING ERROR: Couldn't write to file: {}", e);
        }
    }
}
