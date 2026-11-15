use std::fmt::Write;
use chrono::{DateTime, FixedOffset, Local, NaiveDate, NaiveDateTime, NaiveTime, SecondsFormat, Utc};
use mlua::{Lua, Result, Table, Variadic};
use std::{fs, io, path::Path};
use std::fs::File;
use std::path::PathBuf;
use std::process::Command;
use walkdir::WalkDir;
use zip::{CompressionMethod, ZipWriter};
use zip::write::SimpleFileOptions;

fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let target = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?; // überschreibt vorhandene Dateien
        }
    }
    Ok(())
}

fn move_dir(src: &Path, dst: &Path) -> io::Result<()> {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    // Schnell: Umbenennen, geht nur auf demselben Laufwerk/Dateisystem
    if fs::rename(src, dst).is_ok() {
        return Ok(());
    }
    // Fallback: kopieren, dann Quelle löschen
    copy_dir_all(src, dst)?;
    fs::remove_dir_all(src)
}

fn zip_dir(src: &Path, dst: &Path) -> anyhow::Result<()> {
    let mut zip = ZipWriter::new(File::create(dst)?);
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    for entry in WalkDir::new(src) {
        let entry = entry?;
        let path = entry.path();
        let rel = path.strip_prefix(src)?;
        if rel.as_os_str().is_empty() {
            continue; // Wurzelverzeichnis selbst
        }
        // ZIP erwartet "/" als Trenner, auch unter Windows
        let name = rel.to_string_lossy().replace('\\', "/");

        if entry.file_type().is_dir() {
            zip.add_directory(name, opts)?;
        } else {
            zip.start_file(name, opts)?;
            io::copy(&mut File::open(path)?, &mut zip)?;
        }
    }
    zip.finish()?;
    Ok(())
}

enum Parsed {
    Zoned(DateTime<FixedOffset>), // Eingabe enthielt einen Offset (%z)
    Naive(NaiveDateTime),
}

fn parse(s: &str, fmt: &str) -> Option<Parsed> {
    if let Ok(d) = DateTime::parse_from_str(s, fmt) {
        return Some(Parsed::Zoned(d));
    }
    if let Ok(d) = NaiveDateTime::parse_from_str(s, fmt) {
        if fmt.contains("%s") {
            return Some(Parsed::Zoned(d.and_utc().fixed_offset()));
        }
        return Some(Parsed::Naive(d));
    }
    NaiveDate::parse_from_str(s, fmt)
        .ok()
        .map(|d| Parsed::Naive(d.and_time(NaiveTime::MIN)))
}


pub fn register(lua: &Lua) -> Result<()> {
    register_fs(lua)?;
    register_time(lua)?;
    register_general(lua)?;

    Ok(())
}
pub fn register_fs(lua: &Lua) -> Result<()> {
    let fs: Table = lua.create_table()?;

    fs.set(
        "ensure_dir",
        lua.create_function(|_, path: String| {
            fs::create_dir_all(&path).map_err(mlua::Error::external)?;
            Ok(true)
        })?,
    )?;

    fs.set(
        "copy_dir",
        lua.create_function(|_, (src, dst): (String, String)| {
            copy_dir_all(Path::new(&src), Path::new(&dst)).map_err(mlua::Error::external)?;
            Ok(true)
        })?,
    )?;

    fs.set(
        "move_dir",
        lua.create_function(|_, (src, dst): (String, String)| {
            move_dir(Path::new(&src), Path::new(&dst)).map_err(mlua::Error::external)?;
            Ok(true)
        })?,
    )?;

    fs.set(
        "list_dirs",
        lua.create_function(|lua, path: String| {
            let mut entries = Vec::new();
            for entry in fs::read_dir(&path).map_err(mlua::Error::external)? {
                let entry = entry.map_err(mlua::Error::external)?;
                if entry.file_type().map_err(mlua::Error::external)?.is_dir() {
                    entries.push((
                        entry.file_name().to_string_lossy().into_owned(),
                        entry.path().to_string_lossy().into_owned(),
                    ));
                }
            }
            entries.sort_by(|a, b| a.0.cmp(&b.0));

            let result = lua.create_table()?;
            for (i, (name, full_path)) in entries.into_iter().enumerate() {
                let t = lua.create_table()?;
                t.set("name", name)?;
                t.set("full_path", full_path)?;
                result.set(i + 1, t)?;
            }
            Ok(result)
        })?,
    )?;

    fs.set(
        "list_files",
            lua.create_function(|lua, path: String| {
                let mut entries = Vec::new();
                for entry in fs::read_dir(&path).map_err(mlua::Error::external)? {
                    let entry = entry.map_err(mlua::Error::external)?;
                    if entry.file_type().map_err(mlua::Error::external)?.is_file() {
                        entries.push((
                            entry.file_name().to_string_lossy().into_owned(),
                            entry.path().to_string_lossy().into_owned(),
                        ));
                    }
                }
                entries.sort_by(|a, b| a.0.cmp(&b.0));

                let result = lua.create_table()?;
                for (i, (name, full_path)) in entries.into_iter().enumerate() {
                    let t = lua.create_table()?;
                    t.set("name", name)?;
                    t.set("full_path", full_path)?;
                    result.set(i + 1, t)?;
                }
                Ok(result)
            })?,
        )?;

    fs.set(
        "exe_path",
        lua.create_function(|lua, path: String| {
            let mut entries = Vec::new();
            for entry in fs::read_dir(&path).map_err(mlua::Error::external)? {
                let entry = entry.map_err(mlua::Error::external)?;
                if entry.file_type().map_err(mlua::Error::external)?.is_file() {
                    entries.push((
                        entry.file_name().to_string_lossy().into_owned(),
                        entry.path().to_string_lossy().into_owned(),
                    ));
                }
            }
            entries.sort_by(|a, b| a.0.cmp(&b.0));

            let result = lua.create_table()?;
            for (i, (name, full_path)) in entries.into_iter().enumerate() {
                let t = lua.create_table()?;
                t.set("name", name)?;
                t.set("full_path", full_path)?;
                result.set(i + 1, t)?;
            }
            Ok(result)
        })?,
    )?;

    fs.set(
        "exe_dir",
        lua.create_function(|_, ()| {
            let p = std::env::current_exe().map_err(mlua::Error::external)?;
            let dir = p.parent().ok_or_else(|| mlua::Error::external("kein Elternordner"))?;
            Ok(dir.to_string_lossy().into_owned())
        })?,
    )?;

    fs.set(
        "path_join",
        lua.create_function(|_, parts: Variadic<String>| {
            let mut p = PathBuf::new();
            for part in parts.iter() {
                p.push(part);
            }
            Ok(p.to_string_lossy().into_owned())
        })?,
    )?;

    fs.set(
        "copy_file",
        lua.create_function(|_, (src, dst): (String, String)| {
            fs::copy(&src, &dst).map_err(mlua::Error::external)?;
            Ok(true)
        })?,
    )?;

    fs.set(
        "remove_dir",
        lua.create_function(|_, path: String| {
            std::fs::remove_dir_all(&path).map_err(mlua::Error::external)
        })?,
    )?;

    fs.set(
        "remove_file",
        lua.create_function(|_, path: String| {
            std::fs::remove_file(&path).map_err(mlua::Error::external)
        })?,
    )?;

    lua.globals().set("fs", fs)?;

    Ok(())
}

pub fn register_time(lua: &Lua) -> Result<()> {
    let time: Table = lua.create_table()?;

    // ISO 8601 mit Millisekunden, z. B. "2026-10-26T14:03:12.345Z"
    time.set(
        "utc_now",
        lua.create_function(|_, ()| {
            Ok(Utc::now().to_rfc3339_opts(SecondsFormat::Micros, true))
        })?,
    )?;

    // Unix-Timestamp in Millisekunden
    time.set(
        "utc_unix",
        lua.create_function(|_, ()| Ok(Utc::now().timestamp_micros()))?,
    )?;

    time.set(
        "unix_to_human",
        lua.create_function(|_, us: i64| {
            let dt = chrono::DateTime::from_timestamp_micros(us)
                .ok_or_else(|| mlua::Error::external("Timestamp außerhalb des gültigen Bereichs"))?;
            Ok(dt.to_rfc3339_opts(SecondsFormat::Micros, true))
        })?,
    )?;

    time.set(
        "db_to_unix",
        lua.create_function(|_, s: String| {
            let dt = chrono::NaiveDateTime::parse_from_str(s.trim(), "%Y-%m-%d %H:%M:%S%.f")
                .map_err(mlua::Error::external)?;
            Ok(dt.and_utc().timestamp_micros())
        })?,
    )?;

    time.set(
        "unix_to_db",
        lua.create_function(|_, us: i64| {
            let dt = chrono::DateTime::from_timestamp_micros(us)
                .ok_or_else(|| mlua::Error::external("Timestamp außerhalb des gültigen Bereichs"))?;
            // %.6f = 6 Nachkommastellen, z. B. ".345000"
            Ok(dt.format("%Y-%m-%d %H:%M:%S%.6f").to_string())
        })?,
    )?;

    //convert_time("2026-11-11 14:30:00", "%Y-%m-%d %H:%M:%S", "%d.%m.%Y %H:%M")  -- "11.11.2026 14:30"
    //convert_time("1794321000", "%s", "%Y-%m-%dT%H:%M:%S")  -- Unix-Timestamp
    //convert_time("2026-07-01T12:00:00+0000", "%Y-%m-%dT%H:%M:%S%z", "%d.%m.%Y %H:%M", true) -- Sommerzeit in Deutschland: "01.07.2026 14:00"
    //convert_time("1794321000.123456", "%s%.f", "%Y-%m-%d %H:%M:%S%.6f")
    time.set(
        "convert",
        lua.create_function(
            |_, (s, from, to, to_local): (String, String, String, Option<bool>)| {
                let p = parse(&s, &from).ok_or_else(|| {
                    mlua::Error::runtime(format!("'{s}' passt nicht zum Format '{from}'"))
                })?;

                let mut out = String::new();
                let r = match p {
                    Parsed::Zoned(d) if to_local.unwrap_or(false) => {
                        // Offset → Systemzeitzone, danach naive Lokalzeit
                        write!(out, "{}", d.with_timezone(&Local).naive_local().format(&to))
                    }
                    Parsed::Zoned(d) => write!(out, "{}", d.format(&to)),
                    Parsed::Naive(d) => write!(out, "{}", d.format(&to)),
                };
                r.map_err(|_| mlua::Error::runtime(format!("Ausgabeformat '{to}' ungültig")))?;
                Ok(out)
            },
        )?,
    )?;

    lua.globals().set("time", time)?;
    Ok(())
}

pub fn register_general(lua: &Lua) -> Result<()> {
    lua.globals().set(
        "run",
        lua.create_function(
            |lua, (exe, args, cwd): (String, Option<Vec<String>>, Option<String>)| {
                let mut cmd = Command::new(&exe);
                cmd.args(args.unwrap_or_default());
                if let Some(dir) = cwd {
                    cmd.current_dir(dir);
                }
                let out = cmd.output().map_err(mlua::Error::external)?;

                let t = lua.create_table()?;
                t.set("code", out.status.code())?;
                t.set("success", out.status.success())?;
                t.set("stdout", String::from_utf8_lossy(&out.stdout).into_owned())?;
                t.set("stderr", String::from_utf8_lossy(&out.stderr).into_owned())?;
                Ok(t)
            },
        )?,
    )?;

    lua.globals().set(
        "zip",
        lua.create_function(|_, (src, dst): (String, String)| {
            zip_dir(Path::new(&src), Path::new(&dst)).map_err(mlua::Error::external)
        })?,
    )?;

    Ok(())
}