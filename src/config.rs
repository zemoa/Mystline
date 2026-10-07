use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("configuration TOML invalide : {0}")]
    Toml(#[from] toml::de::Error),
    #[error("impossible de sérialiser la configuration : {0}")]
    Serialize(#[from] toml::ser::Error),
    #[error("répertoire de configuration utilisateur introuvable")]
    NoUserDirectory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub tasks_file: PathBuf,
    pub autostart: bool,
    pub shortcuts: Shortcuts,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Shortcuts {
    pub capture: String,
    pub panel: String,
}

impl Shortcuts {
    pub fn distinct(&self) -> bool {
        match (parse_shortcut(&self.capture), parse_shortcut(&self.panel)) {
            (Ok(capture), Ok(panel)) => capture != panel,
            _ => self.capture.trim() != self.panel.trim(),
        }
    }

    pub fn changed(&self, capture: bool, value: &str) -> Result<Self, String> {
        parse_shortcut(value)?;
        let mut updated = self.clone();
        if capture {
            updated.capture = value.trim().into();
        } else {
            updated.panel = value.trim().into();
        }
        if updated.distinct() {
            Ok(updated)
        } else {
            Err("Les deux actions doivent utiliser des raccourcis différents.".into())
        }
    }
}

pub fn parse_shortcut(value: &str) -> Result<global_hotkey::hotkey::HotKey, String> {
    let value = value
        .split('+')
        .map(str::trim)
        .map(|part| {
            if part.eq_ignore_ascii_case("espace") {
                "Space"
            } else {
                part
            }
        })
        .collect::<Vec<_>>()
        .join("+");
    value
        .parse()
        .map_err(|_| "combinaison invalide (exemple : Ctrl+Shift+Espace)".into())
}

impl Default for Shortcuts {
    fn default() -> Self {
        Self {
            capture: "Ctrl+Shift+Espace".into(),
            panel: "Ctrl+Shift+T".into(),
        }
    }
}

pub fn directory() -> Result<PathBuf, ConfigError> {
    ProjectDirs::from("", "", "Mystline")
        .map(|dirs| dirs.config_dir().to_path_buf())
        .ok_or(ConfigError::NoUserDirectory)
}

pub fn load_or_create(dir: &Path) -> Result<Config, ConfigError> {
    fs::create_dir_all(dir)?;
    let path = dir.join("config.toml");
    match fs::read_to_string(&path) {
        Ok(text) => Ok(toml::from_str(&text)?),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let config = Config {
                tasks_file: dir.join("tasks.md"),
                autostart: true,
                shortcuts: Shortcuts::default(),
            };
            save(dir, &config)?;
            Ok(config)
        }
        Err(e) => Err(e.into()),
    }
}

pub fn save(dir: &Path, config: &Config) -> Result<(), ConfigError> {
    let mut temp = tempfile::NamedTempFile::new_in(dir)?;
    temp.write_all(toml::to_string_pretty(config)?.as_bytes())?;
    temp.as_file().sync_all()?;
    let dest = dir.join("config.toml");
    temp.persist(dest).map_err(|e| e.error)?;
    Ok(())
}

pub struct InstanceLock(File);

impl InstanceLock {
    pub fn acquire(dir: &Path) -> io::Result<Option<Self>> {
        use fs2::FileExt;
        fs::create_dir_all(dir)?;
        let file = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(dir.join("instance.lock"))?;
        match file.try_lock_exclusive() {
            Ok(()) => Ok(Some(Self(file))),
            // Windows reports a contended LockFileEx as ERROR_LOCK_VIOLATION (33),
            // which Rust currently classifies as Uncategorized instead of WouldBlock.
            Err(e)
                if e.kind() == io::ErrorKind::WouldBlock
                    || (cfg!(windows) && e.raw_os_error() == Some(33)) =>
            {
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }
}

impl Drop for InstanceLock {
    fn drop(&mut self) {
        let _ = fs2::FileExt::unlock(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn create_and_persist_config() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = load_or_create(dir.path()).unwrap();
        assert!(config.autostart);
        config.autostart = false;
        save(dir.path(), &config).unwrap();
        assert!(!load_or_create(dir.path()).unwrap().autostart);
    }
    #[test]
    fn shortcuts_accept_french_space_and_detect_equivalent_collision() {
        let shortcuts = Shortcuts {
            capture: "Ctrl + Shift + Espace".into(),
            panel: "shift+control+Space".into(),
        };
        assert_eq!(
            parse_shortcut(&shortcuts.capture).unwrap(),
            parse_shortcut(&shortcuts.panel).unwrap()
        );
        assert!(!shortcuts.distinct());
        assert!(parse_shortcut("Ctrl+Shift+").is_err());
    }
    #[test]
    fn changing_one_shortcut_preserves_the_other_and_persists() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = load_or_create(dir.path()).unwrap();
        config.shortcuts = config.shortcuts.changed(true, "Alt+Shift+C").unwrap();
        save(dir.path(), &config).unwrap();
        let reopened = load_or_create(dir.path()).unwrap();
        assert_eq!(reopened.shortcuts.capture, "Alt+Shift+C");
        assert_eq!(reopened.shortcuts.panel, "Ctrl+Shift+T");
    }
    #[test]
    fn collision_refuses_the_whole_change() {
        let shortcuts = Shortcuts::default();
        assert!(shortcuts.changed(true, "Shift+Ctrl+T").is_err());
        assert_eq!(shortcuts, Shortcuts::default());
    }
    #[test]
    fn instance_is_exclusive() {
        let dir = tempfile::tempdir().unwrap();
        let guard = InstanceLock::acquire(dir.path()).unwrap().unwrap();
        assert!(InstanceLock::acquire(dir.path()).unwrap().is_none());
        drop(guard);
        assert!(InstanceLock::acquire(dir.path()).unwrap().is_some());
    }
}
