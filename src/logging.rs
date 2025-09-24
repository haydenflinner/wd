use std::path::PathBuf;

use anyhow::{Context, Result};
use colored::Colorize;
use directories::ProjectDirs;
use env_logger;
use tracing_subscriber::{
    self, filter::EnvFilter, prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt,
    Layer,
};

pub fn initialize_logging() -> Result<()> {
    let directory = if let Ok(s) = std::env::var("RATATUI_TEMPLATE_DATA") {
        PathBuf::from(s)
    } else if let Some(proj_dirs) = ProjectDirs::from("com", "kdheepak", "ratatui-template") {
        proj_dirs.data_local_dir().to_path_buf()
    } else {
        let s = "Error".red().bold();
        eprintln!("{s}: Unable to find data directory for ratatui-template");
        std::process::exit(1)
    };
    std::fs::create_dir_all(directory.clone())
        .context(format!("{directory:?} could not be created"))?;
    let log_path = directory.join("ratatui-template-debug.log");
    let log_file = std::fs::File::create(log_path)?;
    let file_subscriber = tracing_subscriber::fmt::layer()
        .with_file(true)
        .with_line_number(true)
        .with_writer(log_file)
        .with_target(false)
        .with_ansi(false)
        .with_filter(EnvFilter::from_default_env());
    tracing_subscriber::registry()
        .with(file_subscriber)
        .with(tui_logger::TuiTracingSubscriberLayer)
        .init();
    let default_level = std::env::var("RUST_LOG").map_or(log::LevelFilter::Info, |val| {
        match val.to_lowercase().as_str() {
            "off" => log::LevelFilter::Off,
            "error" => log::LevelFilter::Error,
            "warn" => log::LevelFilter::Warn,
            "info" => log::LevelFilter::Info,
            "debug" => log::LevelFilter::Debug,
            "trace" => log::LevelFilter::Trace,
            _ => {
                let module_directive: Vec<&str> = val.split('=').collect();
                if module_directive.len() == 2 && module_directive[0] == "ratatui-template" {
                    match module_directive[1].to_lowercase().as_str() {
                        "off" => log::LevelFilter::Off,
                        "error" => log::LevelFilter::Error,
                        "warn" => log::LevelFilter::Warn,
                        "info" => log::LevelFilter::Info,
                        "debug" => log::LevelFilter::Debug,
                        "trace" => log::LevelFilter::Trace,
                        _ => log::LevelFilter::Info,
                    }
                } else {
                    log::LevelFilter::Info
                }
            }
        }
    });
    tui_logger::set_default_level(default_level);
    Ok(())
}
