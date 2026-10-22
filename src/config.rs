use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{
    fs,
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
    #[serde(default = "show_tasks_on_startup_default")]
    pub show_tasks_on_startup: bool,
    pub shortcuts: Shortcuts,
}

fn show_tasks_on_startup_default() -> bool {
    true
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
        self.changed_with(capture, value, |value| {
            parse_shortcut(value).map(|key| key.to_string())
        })
    }

    pub fn changed_for_portal(&self, capture: bool, value: &str) -> Result<Self, String> {
        self.changed_with(capture, value, portal_shortcut)
    }

    fn changed_with(
        &self,
        capture: bool,
        value: &str,
        normalize: impl Fn(&str) -> Result<String, String>,
    ) -> Result<Self, String> {
        let requested = normalize(value)?;
        let mut updated = self.clone();
        if capture {
            updated.capture = value.trim().into();
        } else {
            updated.panel = value.trim().into();
        }
        let other = if capture { &self.panel } else { &self.capture };
        if normalize(other).ok().as_ref() != Some(&requested) {
            Ok(updated)
        } else {
            Err("Les deux actions doivent utiliser des raccourcis différents.".into())
        }
    }
}

/// Convert a configured shortcut to the XDG shortcuts specification.
/// Symbols denote keysyms, never positions inferred from an AZERTY/QWERTY layout.
pub fn portal_shortcut(value: &str) -> Result<String, String> {
    use global_hotkey::hotkey::Modifiers;

    let combination = value.rsplit_once('+');
    let key = combination.map_or(value, |(_, key)| key);
    let symbol = match key.trim() {
        "!" | "exclam" => Some("exclam"),
        "ù" | "ugrave" => Some("ugrave"),
        _ => None,
    };
    // Reuse the native parser to validate the modifier list, including ordering.
    let hotkey = if symbol.is_some() {
        parse_shortcut(
            &combination.map_or_else(|| "A".to_owned(), |(prefix, _)| format!("{prefix}+A")),
        )?
    } else {
        parse_shortcut(value)?
    };
    let mut parts = Vec::new();
    for (modifier, name) in [
        (Modifiers::CONTROL, "CTRL"),
        (Modifiers::SHIFT, "SHIFT"),
        (Modifiers::ALT, "ALT"),
        (Modifiers::SUPER, "LOGO"),
    ] {
        if hotkey.mods.contains(modifier) {
            parts.push(name.to_owned());
        }
    }
    let key = hotkey.key.to_string();
    parts.push(
        symbol
            .map(str::to_owned)
            .unwrap_or_else(|| key.strip_prefix("Key").unwrap_or(&key).to_ascii_lowercase()),
    );
    Ok(parts.join("+"))
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
                show_tasks_on_startup: true,
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn portal_accepts_and_persists_french_symbol_shortcuts() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = load_or_create(dir.path()).unwrap();
        config.shortcuts = config
            .shortcuts
            .changed_for_portal(true, "Ctrl + !")
            .unwrap();
        config.shortcuts = config
            .shortcuts
            .changed_for_portal(false, "Ctrl + ù")
            .unwrap();
        save(dir.path(), &config).unwrap();
        let reopened = load_or_create(dir.path()).unwrap();
        assert_eq!(reopened.shortcuts.capture, "Ctrl + !");
        assert_eq!(reopened.shortcuts.panel, "Ctrl + ù");
        assert_eq!(
            portal_shortcut(&reopened.shortcuts.capture).unwrap(),
            "CTRL+exclam"
        );
        assert_eq!(
            portal_shortcut(&reopened.shortcuts.panel).unwrap(),
            "CTRL+ugrave"
        );
        assert!(
            reopened
                .shortcuts
                .changed_for_portal(false, "Control+exclam")
                .is_err()
        );
        assert!(
            reopened
                .shortcuts
                .changed_for_portal(true, "Control+ugrave")
                .is_err()
        );
        for invalid in [
            "Ctrl++!",
            "+!",
            "Ctrl+A+ù",
            "Ctrl+ù+Shift",
            "Ctrl+",
            "Ctrl+unknown",
        ] {
            assert!(portal_shortcut(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn create_and_persist_config() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = load_or_create(dir.path()).unwrap();
        assert!(config.autostart);
        assert!(config.show_tasks_on_startup);
        config.autostart = false;
        config.show_tasks_on_startup = false;
        save(dir.path(), &config).unwrap();
        let reopened = load_or_create(dir.path()).unwrap();
        assert!(!reopened.autostart);
        assert!(!reopened.show_tasks_on_startup);
    }
    #[test]
    fn existing_config_opens_tasks_by_default_independently_of_autostart() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("config.toml"),
            "tasks_file = 'tasks.md'\nautostart = false\n[shortcuts]\ncapture = 'Ctrl+Shift+Espace'\npanel = 'Ctrl+Shift+T'\n",
        )
        .unwrap();
        let config = load_or_create(dir.path()).unwrap();
        assert!(config.show_tasks_on_startup);
        assert!(!config.autostart);
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
}
