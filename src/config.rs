use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub script: PathBuf,
    pub log: Option<LogConfig>,
    pub odbc: Option<OdbcConfig>,
    pub ftp: Option<FtpConfig>,
}

#[derive(Debug, Deserialize)]
pub struct LogConfig {
    pub path: Option<PathBuf>,
    #[serde(default)]
    pub level: LogLevel,
}

#[derive(Debug, Deserialize)]
pub struct ServiceConfig {
    interval: i64,
    #[serde(default)]
    pause: i64,
}

#[derive(Debug, Deserialize)]
pub struct OdbcConfig {
    #[serde(default)]
    pub level: LogLevel,
}

#[derive(Debug, Deserialize)]
pub struct FtpConfig {
    #[serde(default)]
    pub level: LogLevel,
}

pub fn load_config() -> anyhow::Result<Config> {
    let dir: PathBuf = std::env::current_exe()?.parent().unwrap().to_path_buf();
    let text = std::fs::read_to_string(dir.join("config.toml"))?;
    Ok(toml::from_str(&text)?)
}