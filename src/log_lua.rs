
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