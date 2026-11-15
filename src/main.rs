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

#[cfg(target_os = "linux")]
use service_linux::svc;

use std::sync::{atomic::{AtomicBool, Ordering}, Arc};
use crate::host::Host;

fn main() {
    let arg = std::env::args().nth(1);
    match arg.as_deref() {
        Some("--install") => svc::install().unwrap(),
        Some("--uninstall") => svc::uninstall().unwrap(),
        Some("--service") => svc::run().unwrap(),
        _ => run_app(Arc::new(AtomicBool::new(false)), false), // Konsolenbetrieb
    }
}


fn run_app(stop: Arc<AtomicBool>, is_service: bool) {
    let report = Host::new();
    if let Ok(mut report) = report {
        if is_service {
            if let Err(e) = report.set_service() {
                tracing::error!("Could not set service: {}", e);
            }
            if let Err(e) = report.install_stop_hook(stop.clone()) {
                tracing::error!("Could not install stop hook: {}", e);
            }

            while !stop.load(Ordering::Relaxed) {
                // Arbeits-Tick, Lua-Host usw.
                if let Err(e) = report.execute() {
                    tracing::error!("Execution failed: {}", e);
                }

                // Pause
                let pause = std::time::Duration::from_secs(900);
                tracing::trace!("Wait for {:?}", pause);
                let deadline = std::time::Instant::now() + pause;
                while !stop.load(Ordering::Relaxed) {
                    if std::time::Instant::now() >= deadline {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            }
        } else {
            // Konsolenausführung
            if let Err(e) = report.execute() {
                tracing::error!("Execution failed: {}", e);
            }
        }
    }

}