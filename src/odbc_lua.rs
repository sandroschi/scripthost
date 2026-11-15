use mlua::{ExternalResult, Lua, MetaMethod, Result, Table, UserData, UserDataMethods};
use odbc_api::{buffers::TextRowSet, ConnectionOptions, Cursor, ResultSetMetadata};

struct OdbcConn {
    conn: Option<odbc_api::Connection<'static>>,
}

impl UserData for OdbcConn {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // conn:query("SELECT ...") -> Liste von Zeilen-Tabellen
        methods.add_method_mut("query", |lua, this, sql: String| {
            let start_time = std::time::Instant::now();
            let rows = lua.create_table()?;

            // Je nach odbc-api-Version hat execute ein drittes Argument (Timeout).
            let conn = &this.conn.as_ref().ok_or_else(||"").into_lua_err()?;
            let cursor = conn.execute(&sql, (), None).into_lua_err()?;
            let Some(mut cursor) = cursor else {
                return Ok(rows); // Statement ohne Ergebnismenge (INSERT usw.)
            };

            let names: Vec<String> = cursor
                .column_names()
                .into_lua_err()?
                .collect::<std::result::Result<_, _>>()
                .into_lua_err()?;

            // Report available columns if sql has * in it
            tracing::debug!("SQL: {}", sql);
            if sql.contains("*") {
                let mut available_columns: Vec<String> = Vec::with_capacity(names.len());
                names.iter().for_each(|name| { available_columns.push(format!("{}", name)); });
                tracing::debug!("Available columns: {}", available_columns.join(", "));
            }

            // Wenn die batch_size hier auf 1000 steht, werden bei 100 Spalten mehrere Gigabyte an RAM hin und her geschubst
            let mut buffers =
                TextRowSet::for_cursor(100, &mut cursor, Some(100_000)).into_lua_err()?;
            let mut block_cursor = cursor.bind_buffer(&mut buffers).into_lua_err()?;

            let mut n = 1;
            while let Some(batch) = block_cursor.fetch().into_lua_err()? {
                for r in 0..batch.num_rows() {
                    let row = lua.create_table()?;
                    for (c, name) in names.iter().enumerate() {
                        // NULL bleibt nil, also fehlt der Schlüssel in der Tabelle
                        if let Some(bytes) = batch.at(c, r) {
                            // In Windows liefert der Treiber Daten mit dieser Codepage (empirisch ermittelt)
                            let (text, _, _) = encoding_rs::WINDOWS_1252.decode(bytes);
                            row.set(name.as_str(), lua.create_string(text.as_bytes())?)?;
                            //row.set(name.as_str(), lua.create_string(bytes)?)?;
                        }
                    }
                    rows.set(n, row)?;
                    n += 1;
                }
            }
            tracing::debug!("Returned {} rows in {:?}", n-1, start_time.elapsed());
            Ok(rows)
        });

        methods.add_method_mut("close", |_, this, ()| {
            this.conn.take(); // close().map_err(|(_, e)| mlua::Error::external(e))?;
            Ok(())
        });

        // automatisches Schließen bei `local db <close> = ...` (Lua 5.4)
        methods.add_meta_method_mut(MetaMethod::Close, |_, this, _: mlua::Value| {
            this.conn.take();
            Ok(())
        });
    }
}

pub fn register(lua: &Lua) -> Result<()> {
    let odbc: Table = lua.create_table()?;

    odbc.set(
        "connect",
        lua.create_function(|_, conn_str: String| {
            let env = odbc_api::environment().into_lua_err()?;
            let conn = env
                .connect_with_connection_string(&conn_str, ConnectionOptions::default())
                .into_lua_err()?;
            Ok(OdbcConn { conn: Some(conn) })
        })?,
    )?;

    lua.globals().set("odbc", odbc)?;
    Ok(())
}