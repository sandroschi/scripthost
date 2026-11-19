use std::{env, fs};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing_appender::non_blocking::WorkerGuard;
use anyhow::{anyhow, Result, Context};
use mlua::{Lua, Table, HookTriggers, VmState};
use crate::{log_lua, odbc_lua, util_lua};
use crate::config::Config;

pub struct Host {
    lua: Lua,
}

impl Host {
    /// Initialisiert alles
    pub fn new(config: &Config) -> Result<Host> {
        // Verzeichnis der laufenden Exe bestimmen
        let exe_dir = env::current_exe()?
            .parent().ok_or(anyhow!("No parent directory"))?
            .to_path_buf();
        tracing::trace!("Executing in {}", exe_dir.display());
        
        // Lua Umgebung erzeugen und Funktionen registrieren
        let lua = Lua::new();
        odbc_lua::register(&lua)?;
        log_lua::register(&lua)?;
        util_lua::register(&lua)?;
        cred_lua::register(&lua)?;

        // Lua Path erweitern, damit require("foo") auch scripts/foo.lua findet
        // Add scripts parent to path
        let package: Table = lua.globals().get("package")?;
        let old_path: String = package.get("path")?;
        if let Some(scripts_dir) = config.config_path.join(config.script.clone().unwrap_or_default()).parent() {
            let new_path = format!(
                "{};{}",
                scripts_dir.join("?.lua").display(),
                old_path
            );
            package.set("path", new_path)?;
        }

        Ok(Host {
            lua,
        })
    }

    /// Main Logic Execution
    pub fn execute(&mut self, script_path: PathBuf) -> Result<()> {
        tracing::trace!("Executing code of {}", script_path.display());
        let code = fs::read_to_string(&script_path).context("Reading configured script")?;

        // Lua ausführen
        // "@" davor sorgt dafür, dass Fehlermeldungen den Dateinamen zeigen
        self.lua.load(&code)
            .set_name(format!("@{}", script_path.display()))
            .exec()?;

        tracing::trace!("Done");
        Ok(())
    }

    /// Setzt Lua Variable, dass das Programm im Service-Kontext ausgeführt wird.
    pub fn set_service(&mut self) -> Result<()> {
        tracing::trace!("running as service");
        Ok(self.lua.globals().set("service", true)?)
    }

    /// Wenn das stop-Flag true wird, soll Lua beendet werden. Hiermit sagen wir, dass alle 1000 Lua Instruktionen
    /// das Flag geprüft werden soll und dann ggfs abgebrochen wird.
    pub fn install_stop_hook(&mut self, stop: Arc<AtomicBool>) -> mlua::error::Result<()> {
        self.lua.set_hook(
            HookTriggers::new().every_nth_instruction(1000),
            move |_lua, _debug| {
                if stop.load(Ordering::Relaxed) {
                    Err(mlua::Error::runtime("service stopping"))
                } else {
                    Ok(VmState::Continue)
                }
            },
        )
    }
}