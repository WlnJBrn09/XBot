//! Shared XBot types and user configuration.

use serde::{Deserialize, Serialize};
use std::{env, fs, io, path::PathBuf};

pub const BUS_NAME: &str = "org.cruxos.XBot1";
pub const OBJECT_PATH: &str = "/org/cruxos/XBot1";
pub const DAEMON_INTERFACE: &str = "org.cruxos.XBot1.Daemon";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Tier {
    T0,
    T1,
    T2,
    T3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IncidentStatus {
    Open,
    Explained,
    Fixing,
    Resolved,
    Ignored,
    Dismissed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: i64,
    pub incident_id: Option<i64>,
    pub ts: String,
    pub source: String,
    pub severity: Severity,
    pub unit: Option<String>,
    pub exe: Option<String>,
    pub pid: Option<u32>,
    pub message: String,
    pub fields: serde_json::Value,
    pub boot_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Incident {
    pub id: i64,
    pub fingerprint: String,
    pub title: String,
    pub source: String,
    pub severity: Severity,
    pub status: IncidentStatus,
    pub first_seen: String,
    pub last_seen: String,
    pub count: u64,
    pub explanation: Option<String>,
    pub resolution: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub id: String,
    pub session_id: String,
    pub tool: String,
    pub args: serde_json::Value,
    pub tier: Tier,
    pub approval_id: Option<String>,
    pub snapshot_set_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub tier: Tier,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub general: GeneralConfig,
    pub models: ModelsConfig,
    pub voice: VoiceConfig,
    pub privacy: PrivacyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub monitoring: bool,
    pub popups: bool,
    pub follow_dnd: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            monitoring: true,
            popups: true,
            follow_dnd: true,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ModelsConfig {
    pub primary: Option<String>,
    pub fast: Option<String>,
    pub vision: Option<String>,
    pub fallbacks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VoiceConfig {
    pub enabled: bool,
    pub wake_word: String,
    pub model: Option<String>,
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            wake_word: "marvin".into(),
            model: None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PrivacyConfig {
    pub screenshots: bool,
    pub web_search: bool,
    pub browser_use: bool,
}

#[derive(Debug)]
pub enum ConfigError {
    MissingHome,
    Read(io::Error),
    Parse(toml::de::Error),
    Write(io::Error),
    Serialize(toml::ser::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingHome => write!(f, "HOME is not set"),
            Self::Read(err) => write!(f, "cannot read XBot configuration: {err}"),
            Self::Parse(err) => write!(f, "invalid XBot configuration: {err}"),
            Self::Write(err) => write!(f, "cannot write XBot configuration: {err}"),
            Self::Serialize(err) => write!(f, "cannot encode XBot configuration: {err}"),
        }
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    pub fn path() -> Result<PathBuf, ConfigError> {
        if let Some(xdg) = env::var_os("XDG_CONFIG_HOME") {
            return Ok(PathBuf::from(xdg).join("xbot/config.toml"));
        }
        let home = env::var_os("HOME").ok_or(ConfigError::MissingHome)?;
        Ok(PathBuf::from(home).join(".config/xbot/config.toml"))
    }

    pub fn load() -> Result<Self, ConfigError> {
        Self::load_from(Self::path()?)
    }

    pub fn load_from(path: PathBuf) -> Result<Self, ConfigError> {
        match fs::read_to_string(path) {
            Ok(raw) => toml::from_str(&raw).map_err(ConfigError::Parse),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(err) => Err(ConfigError::Read(err)),
        }
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        self.save_to(Self::path()?)
    }

    /// Writes the configuration atomically: a temporary file in the same
    /// directory is renamed over the old one.
    pub fn save_to(&self, path: PathBuf) -> Result<(), ConfigError> {
        let raw = toml::to_string_pretty(self).map_err(ConfigError::Serialize)?;
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(ConfigError::Write)?;
        }
        let tmp = path.with_extension("toml.tmp");
        fs::write(&tmp, raw).map_err(ConfigError::Write)?;
        fs::rename(&tmp, &path).map_err(ConfigError::Write)
    }
}

#[derive(Debug, Serialize)]
pub struct Status {
    pub schema: &'static str,
    pub version: &'static str,
    pub monitoring: bool,
    pub monitoring_requested: bool,
    pub enabled_detectors: Vec<&'static str>,
    pub providers: Vec<&'static str>,
    pub voice_enabled: bool,
    pub phase: &'static str,
}

impl Status {
    pub fn from_config(config: &Config) -> Self {
        Self {
            schema: "xbot.status/1",
            version: env!("CARGO_PKG_VERSION"),
            monitoring: false,
            monitoring_requested: config.general.monitoring,
            enabled_detectors: vec![],
            providers: vec![],
            voice_enabled: false,
            phase: "X0",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_private() {
        let config = Config::default();
        assert!(!config.voice.enabled);
        assert!(!config.privacy.screenshots);
        assert!(!config.privacy.web_search);
        assert!(!config.privacy.browser_use);
    }

    #[test]
    fn partial_config_keeps_safe_defaults() {
        let config: Config = toml::from_str("[general]\nmonitoring = false\n").unwrap();
        assert!(!config.general.monitoring);
        assert!(!config.voice.enabled);
        assert!(!config.privacy.web_search);
    }

    #[test]
    fn saved_config_round_trips() {
        let dir = env::temp_dir().join(format!("xbot-core-test-{}", std::process::id()));
        let path = dir.join("xbot/config.toml");
        let mut config = Config::default();
        config.general.popups = false;
        config.privacy.web_search = true;
        config.save_to(path.clone()).unwrap();
        let loaded = Config::load_from(path).unwrap();
        assert!(!loaded.general.popups);
        assert!(loaded.privacy.web_search);
        assert!(!loaded.voice.enabled);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn status_has_versioned_schema() {
        let value = serde_json::to_value(Status::from_config(&Config::default())).unwrap();
        assert_eq!(value["schema"], "xbot.status/1");
        assert_eq!(value["phase"], "X0");
    }
}
