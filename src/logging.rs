use std::path::PathBuf;
use anyhow::{Context,Result};
use toml::Value::String;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
use crate::config::Config;
use file_rotate::{compression::Compression, suffix::AppendCount, ContentLimit, FileRotate};


pub fn init_logging(config: &Config) -> anyhow::Result<Option<WorkerGuard>> {
    let mut log_folder: Option<PathBuf> = None;
    if let Some(path) = config.log.path.as_ref() {
        log_folder = Some(path.clone())
    } else {
        if let Ok(path) = get_log_folder().context("Getting default log folder") {
            log_folder = Some(path);
        }
    }

    let filter_string = format!(
        "{},odbc_api={},suppaftp={},{}",
        config.log.log.to_string(),
        config.odbc.log.to_string(),
        config.ftp.log.to_string(),
        "calloop=warn,zbus=warn,sctk=warn,arboard=warn,eframe=warn,winit=warn,wgpu=warn,naga=warn,wgpu_core=warn,wgpu_hal=warn"
    );

    if let Some(log_folder) = log_folder.as_ref() {
        let folder_creation = std::fs::create_dir_all(&log_folder); //.with_context(|| format!("failed to read config file: {:?}", log_folder));; //.context(format!("Failed to create log folder {:?}", log_folder));

        if folder_creation.is_ok() {
            let file = FileRotate::new(
                log_folder.join("scripthost.log"),
                AppendCount::new(config.log.max_files),
                ContentLimit::Bytes(10 * 1024 * 1024),  // 10 MB
                Compression::OnRotate(0),               // sofort packen
                //#[cfg(unix)]
                None,
            );

            // Roll the log file every day.
            // let file_appender = tracing_appender::rolling::daily(
            //     &log_folder,
            //     "scripthost.log",
            // );

            // Non-blocking writer for the file.
            // IMPORTANT: keep _guard alive for the lifetime of the application.
            let (file_writer, _guard) =
                tracing_appender::non_blocking(file);

            let console_layer = fmt::layer()
                .with_target(true);

            let file_layer = fmt::layer()
                .with_ansi(false)
                .with_target(true)
                .with_writer(file_writer);

            tracing_subscriber::registry()
                .with(
                    EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| EnvFilter::new(filter_string)),
                )
                .with(console_layer)
                .with(file_layer)
                .init();
            return Ok(Some(_guard));
        } else {
            let console_layer = fmt::layer()
                .with_target(true);

            tracing_subscriber::registry()
                .with(
                    EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| EnvFilter::new(filter_string)),
                )
                .with(console_layer)
                .init();

            tracing::error!(error = folder_creation.unwrap_err().to_string().as_str(), "Failed to create log folder {:?}", log_folder);

            return Ok(None)
        }
    } else {
        let console_layer = fmt::layer()
            .with_target(true);

        tracing_subscriber::registry()
            .with(
                EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| EnvFilter::new(filter_string)),
            )
            .with(console_layer)
            .init();

        return Ok(None)
    }
}

#[cfg(target_os = "windows")]
fn get_log_folder() -> Result<PathBuf> {
    Ok(PathBuf::from(
        std::env::var_os("PROGRAMDATA")
            .context("PROGRAMDATA is not set")?,
    )
        .join("scripthost"))
}

#[cfg(target_os = "linux")]
fn get_log_folder() -> Result<PathBuf> {
    Ok(PathBuf::from("/var/log")
        .join("scripthost"))
}

