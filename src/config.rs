use serde::Deserialize;
use std::path::PathBuf;
use anyhow::Context;

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

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub script: Option<PathBuf>,
    #[serde(default)]
    pub log: LogConfig,
    pub service: Option<ServiceConfig>,
    #[serde(default)]
    pub odbc: OdbcConfig,
    #[serde(default)]
    pub ftp: FtpConfig,
    #[serde(default)]
    pub is_service: bool,

    #[serde(default)]
    pub config_path: PathBuf, // Wird intern gesetzt 
}

#[derive(Debug, Deserialize,Default, Clone)]
pub struct LogConfig {
    pub path: Option<PathBuf>,
    #[serde(default)]
    pub log: LogLevel,
    #[serde(default)]
    pub max_files: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServiceConfig {
    pub name: String, // used to register service - mandatory if service is used, do not use spaces
    //#[serde(default, with = "humantime_serde")]
    //pub interval: std::time::Duration, // interval to reexecute script in service mode - immediately if not defined
    #[serde(default, with = "humantime_serde")]
    pub pause: std::time::Duration, // pause after each execution - format example: 1h 30m
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct OdbcConfig {
    #[serde(default)]
    pub log: LogLevel,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct FtpConfig {
    #[serde(default)]
    pub log: LogLevel,
}

pub fn load_config() -> anyhow::Result<Config> {
    let dir: PathBuf = std::env::current_exe()?.parent().unwrap().to_path_buf();
    let text = std::fs::read_to_string(dir.join("config.toml")).context("No config.toml found")?;
    let mut config: Config = toml::from_str(&text).context("Failed to parse config.toml")?;
    config.config_path = dir;
    Ok(config)
}

impl LogLevel {
    pub fn to_string(&self) -> String {
        match self {
            LogLevel::Error => { String::from("error") }
            LogLevel::Warn => { String::from("warn") }
            LogLevel::Info => { String::from("info") }
            LogLevel::Debug => { String::from("debug") }
            LogLevel::Trace => { String::from("trace") }
        }
    }
}