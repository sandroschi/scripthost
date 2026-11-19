
use mlua::{Lua, Result, Table};
use std::path::PathBuf;

#[cfg(unix)]
mod backend {
    use mlua::{Error, Result};
    use std::{
        fs, io, io::Write,
        os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt},
        path::Path,
    };

    // Verhindert Pfadtricks wie "../" in service/user
    fn name(service: &str, user: &str) -> Result<String> {
        let ok = |s: &str| {
            !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        };
        if ok(service) && ok(user) {
            Ok(format!("{service}.{user}"))
        } else {
            Err(Error::runtime("service/user: nur Buchstaben, Ziffern, '-' und '_' erlaubt"))
        }
    }

    fn read(path: &Path) -> Result<Option<String>> {
        match fs::read_to_string(path) {
            Ok(s) => Ok(Some(s.trim_end_matches(['\r', '\n']).to_string())),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(Error::external(e)),
        }
    }

    pub fn get(dir: &Path, service: &str, user: &str) -> Result<Option<String>> {
        let n = name(service, user)?;
        // 1. systemd-Credentials (LoadCredentialEncrypted=<service>.<user>:...)
        if let Some(cd) = std::env::var_os("CREDENTIALS_DIRECTORY") {
            if let Some(v) = read(&Path::new(&cd).join(&n))? {
                return Ok(Some(v));
            }
        }
        // 2. Datei im eigenen Verzeichnis
        read(&dir.join(n))
    }

    pub fn set(dir: &Path, service: &str, user: &str, secret: &str) -> Result<()> {
        let path = dir.join(name(service, user)?);
        fs::DirBuilder::new().recursive(true).mode(0o700).create(dir).map_err(Error::external)?;
        let mut f = fs::OpenOptions::new()
            .write(true).create(true).truncate(true).mode(0o600)
            .open(&path).map_err(Error::external)?;
        // falls die Datei schon mit anderen Rechten existierte
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).map_err(Error::external)?;
        f.write_all(secret.as_bytes()).map_err(Error::external)
    }

    pub fn delete(dir: &Path, service: &str, user: &str) -> Result<()> {
        match fs::remove_file(dir.join(name(service, user)?)) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(Error::external(e)),
        }
    }
}

#[cfg(windows)]
mod backend {
    use keyring::{Entry, Error as KrError};
    use mlua::{Error, Result};
    use std::path::Path;

    pub fn get(_dir: &Path, service: &str, user: &str) -> Result<Option<String>> {
        match Entry::new(service, user).and_then(|e| e.get_password()) {
            Ok(p) => Ok(Some(p)),
            Err(KrError::NoEntry) => Ok(None),
            Err(e) => Err(Error::external(e)),
        }
    }
    pub fn set(_dir: &Path, service: &str, user: &str, secret: &str) -> Result<()> {
        Entry::new(service, user).and_then(|e| e.set_password(secret)).map_err(Error::external)
    }
    pub fn delete(_dir: &Path, service: &str, user: &str) -> Result<()> {
        match Entry::new(service, user).and_then(|e| e.delete_credential()) {
            Ok(()) | Err(KrError::NoEntry) => Ok(()),
            Err(e) => Err(Error::external(e)),
        }
    }
}

pub fn register(lua: &Lua, dir: PathBuf) -> Result<()> {
    let cred: Table = lua.create_table()?;

    let d = dir.clone();
    cred.set("cred_get", lua.create_function(move |_, (s, u): (String, String)| {
        backend::get(&d, &s, &u)
    })?)?;

    let d = dir.clone();
    cred.set("cred_set", lua.create_function(move |_, (s, u, p): (String, String, String)| {
        backend::set(&d, &s, &u, &p)
    })?)?;

    let d = dir;
    cred.set("cred_delete", lua.create_function(move |_, (s, u): (String, String)| {
        backend::delete(&d, &s, &u)
    })?)?;

    lua.globals().set("cred", cred)?;
    Ok(())
}