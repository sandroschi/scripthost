mod odbc_lua;
mod logging;
mod util_lua;
mod host;
#[cfg(target_os = "windows")]
mod service_windows;
#[cfg(target_os = "windows")]
use service_windows::svc;

#[cfg(target_os = "linux")]
mod service_linux;
mod config;
mod help;
mod ftp_lua;
mod log_lua;
mod cred_lua;

#[cfg(target_os = "linux")]
use service_linux::svc;

use std::sync::{atomic::{AtomicBool, Ordering}, Arc};
use crate::config::{load_config, Config};
use crate::host::Host;
use crate::logging::init_logging;

fn main() {
    if let Err(e) = run() {
        if tracing::dispatcher::has_been_set() {
            tracing::error!(error = ?e, "Error");
        } else {
            println!("Error: {e:?}");
        }
    }
}
fn run() -> anyhow::Result<()> {
    let arg = std::env::args().nth(1);
    match arg.as_deref() {
        Some("--install") => svc::install()?,
        Some("--uninstall") => svc::uninstall()?,
        Some("--service") => svc::run()?,
        _ => _ = run_app(Arc::new(AtomicBool::new(false)), false)?, // Konsolenbetrieb
    }
    Ok(())
}


fn run_app(stop: Arc<AtomicBool>, is_service: bool) -> anyhow::Result<()> {
    tracing::trace!("Start");
    let mut config = load_config()?;
    config.is_service = is_service;
    
    let _log_guard = init_logging(&config); 
    let mut host = Host::new(&config)?;

    if let Some(script_path) = config.script.as_ref() {
        // If script_path is relative, make it absolute with config path
        let script_path = config.config_path.join(script_path);

        if is_service {
            if let Err(e) = host.set_service() {
                tracing::error!("Could not set service: {e:#}");
            }
            if let Err(e) = host.install_stop_hook(stop.clone()) {
                tracing::error!("Could not install stop hook: {e:#}");
            }

            while !stop.load(Ordering::Relaxed) {
                // Arbeits-Tick, Lua-Host usw.
                if let Err(e) = host.execute(script_path.clone()) {
                    tracing::error!("Execution failed: {e:#}");
                }

                // Pause
                if let Some(service) = config.service.as_ref() {
                    tracing::trace!("Wait for {:?}", service.pause);
                    let deadline = std::time::Instant::now() + service.pause;
                    while !stop.load(Ordering::Relaxed) {
                        if std::time::Instant::now() >= deadline {
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(50));
                    }
                }
            }
        } else {
            // Konsolenausführung
            if let Err(e) = host.execute(script_path.clone()) {

                tracing::error!("Execution failed: {e:#}");
            }
        }
    } else {
        tracing::error!("No script provided. Nothing to do.");
    }
    
    Ok(())
}