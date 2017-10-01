use log::{self, Log, LogLevelFilter, LogLevel, LogMetadata, LogRecord, SetLoggerError};
use colored::Colorize;
use atty;
use std::io;
use std::io::Write;

struct Logger {
    log_level: LogLevelFilter,
    colours: bool
}

impl Log for Logger {
    fn enabled(&self, metadata: &LogMetadata) -> bool {
        metadata.level() <= self.log_level
    }

    fn log(&self, record: &LogRecord) {
        if self.enabled(record.metadata()) {

            let raw = match record.level() {
                LogLevel::Error => format!("ERROR: {}", record.args()),
                LogLevel::Warn => format!("WARN: {}", record.args()),
                _ => format!("{}", record.args())
            };

            let coloured = if self.colours {
                match record.level() {
                    LogLevel::Error => raw.red().bold(),
                    LogLevel::Warn => raw.yellow().bold(),
                    LogLevel::Info => raw.white().bold(),
                    LogLevel::Debug => raw.white(),
                    LogLevel::Trace => raw.cyan()
                }
            } else {
                raw.normal()
            };

            if record.level() <= LogLevel::Warn {
                let _ = writeln!(&mut io::stderr(), "{}", coloured);
            } else {
                println!("{}", coloured);
            }
        }
    }
}

pub fn init_with_level(log_level: LogLevelFilter) -> Result<(), SetLoggerError> {
    let colours = atty::is(atty::Stream::Stdout) && atty::is(atty::Stream::Stderr);

    log::set_logger(|max_log_level| {
        max_log_level.set(log_level);
        Box::new(Logger {
            log_level: log_level,
            colours: colours
        })
    })
}
