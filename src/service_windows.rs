pub mod svc {
    use crate::Ordering;
    use std::{ffi::OsString, time::Duration};
    use windows_service::{
        define_windows_service,
        service::*,
        service_control_handler::{self, ServiceControlHandlerResult},
        service_dispatcher,
        service_manager::{ServiceManager, ServiceManagerAccess},
    };
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    const NAME: &str = "BatchReportS";
    const TYPE: ServiceType = ServiceType::OWN_PROCESS;

    define_windows_service!(ffi_service_main, service_main);

    // Wird mit --service aufgerufen (vom SCM gestartet)
    pub fn run() -> windows_service::Result<()> {
        service_dispatcher::start(NAME, ffi_service_main)
    }

    fn service_main(_args: Vec<OsString>) {
        if let Err(e) = run_service() {
            eprintln!("Service-Fehler: {e}"); // besser: in Logdatei schreiben
        }
    }

    fn status(state: ServiceState, accept: ServiceControlAccept) -> ServiceStatus {
        ServiceStatus {
            service_type: TYPE,
            current_state: state,
            controls_accepted: accept,
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None,
        }
    }

    fn run_service() -> windows_service::Result<()> {
        // in run_service(), statt der recv_timeout-Schleife:
        let stop = Arc::new(AtomicBool::new(false));
        let stop_h = stop.clone();
        let handle = service_control_handler::register(NAME, move |ev| match ev {
            ServiceControl::Stop => {
                stop_h.store(true, Ordering::Relaxed);
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        })?;
        handle.set_service_status(status(ServiceState::Running, ServiceControlAccept::STOP))?;
        crate::run_app(stop, true);
        handle.set_service_status(status(ServiceState::Stopped, ServiceControlAccept::empty()))?;
        Ok(())
    }

    pub fn install() -> windows_service::Result<()> {
        let manager =
            ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CREATE_SERVICE)?;
        let info = ServiceInfo {
            name: OsString::from(NAME),
            display_name: OsString::from("Batch Report S"),
            service_type: TYPE,
            start_type: ServiceStartType::AutoStart,
            error_control: ServiceErrorControl::Normal,
            executable_path: std::env::current_exe().unwrap(),
            launch_arguments: vec![OsString::from("--service")],
            dependencies: vec![],
            account_name: None, // None = LocalSystem
            account_password: None,
        };
        let s = manager.create_service(&info, ServiceAccess::CHANGE_CONFIG)?;
        s.set_description("Batch Report Generation")?;
        Ok(())
    }

    pub fn uninstall() -> windows_service::Result<()> {
        let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)?;
        let s = manager.open_service(
            NAME,
            ServiceAccess::DELETE | ServiceAccess::STOP | ServiceAccess::QUERY_STATUS,
        )?;
        if s.query_status()?.current_state != ServiceState::Stopped {
            s.stop()?;
        }
        s.delete()
    }
}