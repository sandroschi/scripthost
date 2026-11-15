use mlua::{ExternalResult, Lua, Table, UserData, UserDataMethods};
use std::io::Cursor;
use odbc_api::ConnectionOptions;
use suppaftp::{types::FileType, FtpStream};

struct Ftp(Option<FtpStream>);

fn ftp<'a>(this: &'a mut Ftp) -> mlua::Result<&'a mut FtpStream> {
    this.0.as_mut().ok_or_else(|| mlua::Error::runtime("FTP-Verbindung geschlossen"))
}

impl UserData for Ftp {
    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method_mut("cwd", |_, this, dir: String| {
            ftp(this)?.cwd(dir).map_err(mlua::Error::external)
        });
        m.add_method_mut("list", |_, this, dir: Option<String>| {
            ftp(this)?.nlst(dir.as_deref()).map_err(mlua::Error::external)
        });
        m.add_method_mut("get", |lua, this, name: String| {
            let buf = ftp(this)?.retr_as_buffer(&name).map_err(mlua::Error::external)?;
            lua.create_string(buf.into_inner()) // Bytes bleiben unverändert
        });
        m.add_method_mut("put", |_, this, (name, data): (String, mlua::String)| {
            let mut rd = Cursor::new(data.as_bytes().to_vec());
            ftp(this)?.put_file(&name, &mut rd).map(|_| ()).map_err(mlua::Error::external)
        });
        m.add_method_mut("remove", |_, this, name: String| {
            ftp(this)?.rm(name).map_err(mlua::Error::external)
        });
        m.add_method_mut("close", |_, this, ()| {
            if let Some(mut s) = this.0.take() {
                let _ = s.quit();
            }
            Ok(())
        });
    }
}

pub fn register(lua: &Lua) -> mlua::Result<()> {
    let ftp: Table = lua.create_table()?;

    ftp.set(
        "connect",
        lua.create_function(|_, (addr, user, pass): (String, String, String)| {
            let mut s = FtpStream::connect(addr).map_err(mlua::Error::external)?;
            s.login(user, pass).map_err(mlua::Error::external)?;
            s.transfer_type(FileType::Binary).map_err(mlua::Error::external)?;
            Ok(Ftp(Some(s)))
        })?,
    )?;

    lua.globals().set("ftp", ftp)?;
    Ok(())
}