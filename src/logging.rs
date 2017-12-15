use log::{Level, Record};
use env_logger::fmt::Formatter;
use std::io::{self, Write};
use colored::Colorize;

pub fn output_log_colour(buf: &mut Formatter, record: &Record) -> io::Result<()> {

    let module = record.module_path().unwrap_or("ROOT");
    let raw = match record.level() {
        Level::Error => format!("[ERROR] {}: {}", module, record.args()),
        Level::Warn => format!("[WARN] {}: {}", module, record.args()),
        _ => format!("{}", record.args())
    };

    let coloured = match record.level() {
        Level::Error => raw.red().bold(),
        Level::Warn => raw.yellow().bold(),
        Level::Info => raw.white().bold(),
        Level::Debug => raw.white(),
        Level::Trace => raw.cyan()
    };

    writeln!(buf, "{}", coloured)
}

pub fn output_log(buf: &mut Formatter, record: &Record) -> io::Result<()> {

    let module = record.module_path().unwrap_or("ROOT");
    let raw = match record.level() {
        Level::Error => format!("[ERROR] {}: {}", module, record.args()),
        Level::Warn => format!("[WARN] {}: {}", module, record.args()),
        _ => format!("{}", record.args())
    };

    writeln!(buf, "{}", raw)
}