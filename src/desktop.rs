use crate::config::Config;
use futures::channel::mpsc::UnboundedSender;
use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
    hotkey::{Code, HotKey, Modifiers},
};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    io,
    path::Path,
    sync::{
        OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};
use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuItem},
};

#[derive(Debug, Clone)]
pub enum Event {
    Capture,
    Panel,
    Settings,
    Quit,
    FileChanged,
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    ShortcutStatus(bool, bool),
}

static EVENTS: OnceLock<UnboundedSender<Event>> = OnceLock::new();
static FILE_CHANGE_PENDING: AtomicBool = AtomicBool::new(false);

pub fn install_events(sender: UnboundedSender<Event>) {
    let _ = EVENTS.set(sender);
}
fn emit(event: Event) {
    if let Some(tx) = EVENTS.get() {
        let _ = tx.unbounded_send(event);
    }
}

pub fn acknowledge_file_change() {
    FILE_CHANGE_PENDING.store(false, Ordering::Release);
}

pub struct Tray {
    _icon: TrayIcon,
}

impl Tray {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let menu = Menu::new();
        menu.append(&MenuItem::with_id("capture", "Capture rapide", true, None))?;
        menu.append(&MenuItem::with_id(
            "panel",
            "Ouvrir / masquer le panneau",
            true,
            None,
        ))?;
        menu.append(&MenuItem::with_id("settings", "Paramètres", true, None))?;
        menu.append(&MenuItem::with_id("quit", "Quitter", true, None))?;
        MenuEvent::set_event_handler(Some(|event: MenuEvent| {
            let action = match event.id.0.as_str() {
                "capture" => Event::Capture,
                "panel" => Event::Panel,
                "settings" => Event::Settings,
                "quit" => Event::Quit,
                _ => return,
            };
            emit(action);
        }));
        let mut pixels = vec![0u8; 32 * 32 * 4];
        for y in 0..32 {
            for x in 0..32 {
                if (6..26).contains(&x) && (6..26).contains(&y) {
                    let pos = (y * 32 + x) * 4;
                    pixels[pos..pos + 4].copy_from_slice(&[87, 111, 233, 255]);
                }
            }
        }
        let icon = Icon::from_rgba(pixels, 32, 32)?;
        let icon = TrayIconBuilder::new()
            .with_icon(icon)
            .with_tooltip("Mystline")
            .with_menu(Box::new(menu))
            .build()?;
        Ok(Self { _icon: icon })
    }
}

pub struct Shortcuts {
    _manager: GlobalHotKeyManager,
    pub capture_active: bool,
    pub panel_active: bool,
}

impl Shortcuts {
    pub fn new() -> Result<Self, global_hotkey::Error> {
        let manager = GlobalHotKeyManager::new()?;
        let capture = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Space);
        let panel = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyT);
        let capture_active = manager.register(capture).is_ok();
        let panel_active = manager.register(panel).is_ok();
        GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
            if event.state != HotKeyState::Pressed {
                return;
            }
            if event.id == capture.id() && capture_active {
                emit(Event::Capture);
            }
            if event.id == panel.id() && panel_active {
                emit(Event::Panel);
            }
        }));
        Ok(Self {
            _manager: manager,
            capture_active,
            panel_active,
        })
    }
}

pub fn is_wayland() -> bool {
    cfg!(target_os = "linux")
        && (std::env::var("XDG_SESSION_TYPE").is_ok_and(|s| s.eq_ignore_ascii_case("wayland"))
            || (std::env::var_os("WAYLAND_DISPLAY").is_some()
                && std::env::var_os("DISPLAY").is_none()))
}

pub fn watch(path: &Path) -> notify::Result<RecommendedWatcher> {
    let mut watcher = notify::recommended_watcher(|event: notify::Result<notify::Event>| {
        if event.is_ok()
            && EVENTS.get().is_some()
            && !FILE_CHANGE_PENDING.swap(true, Ordering::AcqRel)
        {
            emit(Event::FileChanged);
        }
    })?;
    // Watch the directory so an atomic replacement of tasks.md remains visible.
    watcher.watch(
        path.parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new(".")),
        RecursiveMode::NonRecursive,
    )?;
    Ok(watcher)
}

#[cfg(target_os = "linux")]
pub fn set_autostart(config: &Config) -> io::Result<()> {
    use std::fs;
    let dir = directories::BaseDirs::new()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME absent"))?
        .config_dir()
        .join("autostart");
    let file = dir.join("mystline.desktop");
    if !config.autostart {
        return match fs::remove_file(file) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            result => result,
        };
    }
    fs::create_dir_all(&dir)?;
    let binary = std::env::current_exe()?;
    let path = binary
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('$', "\\$")
        .replace('`', "\\`");
    let content = format!(
        "[Desktop Entry]\nType=Application\nName=Mystline\nExec=\"{path}\"\nTerminal=false\nX-GNOME-Autostart-enabled=true\n"
    );
    fs::write(file, content)
}

#[cfg(windows)]
pub fn set_autostart(config: &Config) -> io::Result<()> {
    use winreg::{
        RegKey,
        enums::{HKEY_CURRENT_USER, KEY_SET_VALUE},
    };
    let key = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(
        "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        KEY_SET_VALUE,
    )?;
    if config.autostart {
        key.set_value(
            "Mystline",
            &format!("\"{}\"", std::env::current_exe()?.display()),
        )
    } else {
        match key.delete_value("Mystline") {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            result => result,
        }
    }
}

#[cfg(target_os = "linux")]
pub async fn portal_shortcuts() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use ashpd::desktop::{
        CreateSessionOptions,
        global_shortcuts::{BindShortcutsOptions, GlobalShortcuts, NewShortcut},
    };
    use futures::{FutureExt, StreamExt};
    let portal = GlobalShortcuts::new().await?;
    let session = portal
        .create_session(CreateSessionOptions::default())
        .await?;
    let activated = portal.receive_activated().await?;
    let shortcuts = [
        NewShortcut::new("capture", "Capture rapide").preferred_trigger(Some("CTRL+SHIFT+space")),
        NewShortcut::new("panel", "Ouvrir le panneau").preferred_trigger(Some("CTRL+SHIFT+t")),
    ];
    let response = portal
        .bind_shortcuts(&session, &shortcuts, None, BindShortcutsOptions::default())
        .await?
        .response()?;
    let mut capture_active = response.shortcuts().iter().any(|s| s.id() == "capture");
    let mut panel_active = response.shortcuts().iter().any(|s| s.id() == "panel");
    emit(Event::ShortcutStatus(capture_active, panel_active));
    let changed = portal.receive_shortcuts_changed().await?;
    futures::pin_mut!(activated, changed);
    loop {
        futures::select! {
            activation = activated.next().fuse() => match activation {
                Some(activation) => match activation.shortcut_id() {
                    "capture" if capture_active => emit(Event::Capture),
                    "panel" if panel_active => emit(Event::Panel),
                    _ => (),
                },
                None => break,
            },
            update = changed.next().fuse() => match update {
                Some(update) => {
                    capture_active = update.shortcuts().iter().any(|s| s.id() == "capture");
                    panel_active = update.shortcuts().iter().any(|s| s.id() == "panel");
                    emit(Event::ShortcutStatus(capture_active, panel_active));
                }
                None => break,
            },
        }
    }
    Ok(())
}
