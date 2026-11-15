use std::path::PathBuf;
use anyhow::{Context};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

pub fn init_logging() -> anyhow::Result<Option<WorkerGuard>> {
    let log_folder = get_log_folder().context("Getting log folder")?;
    let folder_creation = std::fs::create_dir_all(&log_folder);//.with_context(|| format!("failed to read config file: {:?}", log_folder));; //.context(format!("Failed to create log folder {:?}", log_folder));

    if folder_creation.is_ok() {
        // Roll the log file every day.
        let file_appender = tracing_appender::rolling::daily(
            &log_folder,
            "batch_report_s.log",
        );

        // Non-blocking writer for the file.
        // IMPORTANT: keep _guard alive for the lifetime of the application.
        let (file_writer, _guard) =
            tracing_appender::non_blocking(file_appender);

        let console_layer = fmt::layer()
            .with_target(true);

        let file_layer = fmt::layer()
            .with_ansi(false)
            .with_target(true)
            .with_writer(file_writer);

        tracing_subscriber::registry()
            .with(
                EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| EnvFilter::new("trace,odbc_api=debug,egui=warn,egui_extras=warn,calloop=warn,zbus=warn,sctk=warn,arboard=warn,eframe=warn,winit=warn,wgpu=warn,naga=warn,wgpu_core=warn,wgpu_hal=warn")),
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
                    .unwrap_or_else(|_| EnvFilter::new("trace,odbc_api=debug,egui=warn,egui_extras=warn,calloop=warn,zbus=warn,sctk=warn,arboard=warn,eframe=warn,winit=warn,wgpu=warn,naga=warn,wgpu_core=warn,wgpu_hal=warn")),
            )
            .with(console_layer)
            .init();

        tracing::error!(error = folder_creation.unwrap_err().to_string().as_str(), "Failed to create log folder {:?}", log_folder);

        return Ok(None)
    }
}

#[cfg(target_os = "windows")]
fn get_log_folder() -> Result<PathBuf> {
    Ok(PathBuf::from(
        std::env::var_os("PROGRAMDATA")
            .context("PROGRAMDATA is not set")?,
    )
        .join("batch_report_s"))
}

#[cfg(target_os = "linux")]
fn get_log_folder() -> Result<PathBuf> {
    Ok(PathBuf::from("/var/log")
        .join("batch_report_s"))
}


use mlua::{Lua, Result, Table};


pub fn register(lua: &Lua) -> mlua::Result<()> {
    let log: Table = lua.create_table()?;

    log.set(
        "trace",
        lua.create_function(|_, msg: String| {
            tracing::trace!("{}", msg);
            Ok(())
        })?,
    )?;

    log.set(
        "debug",
        lua.create_function(|_, msg: String| {
            tracing::debug!("{}", msg);
            Ok(())
        })?,
    )?;

    log.set(
        "info",
        lua.create_function(|_, msg: String| {
            tracing::info!("{}", msg);
            Ok(())
        })?,
    )?;

    log.set(
        "warn",
        lua.create_function(|_, msg: String| {
            tracing::warn!("{}", msg);
            Ok(())
        })?,
    )?;

    log.set(
        "error",
        lua.create_function(|_, msg: String| {
            tracing::error!("{}", msg);
            Ok(())
        })?,
    )?;


    lua.globals().set("log", log)?;
    Ok(())
}