pub mod svc {
    use super::*;
    use std::{fs, path::Path, process::Command};
    use std::io::ErrorKind;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    //const NAME: &str = "BatchReportS";

    // Wird mit --service (oder einfach ohne Argument) aufgerufen
    pub fn run() -> std::io::Result<()> {
        let stop = Arc::new(AtomicBool::new(false));
        signal_hook::flag::register(signal_hook::consts::SIGTERM, stop.clone())?;
        signal_hook::flag::register(signal_hook::consts::SIGINT, stop.clone())?;
        _ = crate::run_app(stop, true);
        Ok(())
    }

    pub fn install() -> std::io::Result<()> {
        let name = crate::config::load_config().map_err(std::io::Error::other)?
            .service.ok_or(std::io::Error::new(ErrorKind::NotFound, "no service section found in configuration"))?
            .name;
        let exe = std::env::current_exe()?;
        let workdir = exe.parent().unwrap();
        let unit = format!(
            "[Unit]\nDescription=Batch Report Generation\nAfter=network.target\n\n\
             [Service]\nExecStart={exe} --service\nWorkingDirectory={wd}\n\
             Restart=on-failure\n\n[Install]\nWantedBy=multi-user.target\n",
            exe = exe.display(),
            wd = workdir.display()
        );
        fs::write(format!("/etc/systemd/system/{name}.service"), unit)?;
        Command::new("systemctl").arg("daemon-reload").status()?;
        Command::new("systemctl").args(["enable", name.as_str()]).status()?;
        Ok(())
    }

    pub fn uninstall() -> std::io::Result<()> {
        let name = crate::config::load_config().map_err(std::io::Error::other)?
            .service.ok_or(std::io::Error::new(ErrorKind::NotFound, "no service section found in configuration"))?
            .name;
        let _ = Command::new("systemctl").args(["disable", "--now", name.as_str()]).status();
        let path = format!("/etc/systemd/system/{name}.service");
        if Path::new(&path).exists() {
            fs::remove_file(path)?;
        }
        Command::new("systemctl").arg("daemon-reload").status()?;
        Ok(())
    }
}