use super::desktop::{Event, emit};
use mystline::config;
use tray_icon::{
    Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuItem},
};

pub struct Tray {
    _icon: TrayIcon,
}

fn tray_action(event: TrayIconEvent) -> Option<Event> {
    match event {
        TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } => Some(Event::OpenPanel),
        _ => None,
    }
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
        TrayIconEvent::set_event_handler(Some(|event| {
            if let Some(action) = tray_action(event) {
                emit(action);
            }
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
            .with_menu_on_left_click(false)
            .build()?;
        Ok(Self { _icon: icon })
    }
}

#[cfg(target_os = "linux")]
pub async fn portal_shortcuts(
    config: config::Shortcuts,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
    let capture_trigger = portal_trigger(&config.capture)?;
    let panel_trigger = portal_trigger(&config.panel)?;
    let shortcuts = [
        NewShortcut::new("capture", "Capture rapide")
            .preferred_trigger(Some(capture_trigger.as_str())),
        NewShortcut::new("panel", "Ouvrir le panneau")
            .preferred_trigger(Some(panel_trigger.as_str())),
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

#[cfg(target_os = "linux")]
fn portal_trigger(value: &str) -> Result<String, String> {
    use global_hotkey::hotkey::Modifiers;
    let hotkey = config::parse_shortcut(value)?;
    let mut parts = Vec::new();
    if hotkey.mods.contains(Modifiers::CONTROL) {
        parts.push("CTRL".to_owned());
    }
    if hotkey.mods.contains(Modifiers::SHIFT) {
        parts.push("SHIFT".to_owned());
    }
    if hotkey.mods.contains(Modifiers::ALT) {
        parts.push("ALT".to_owned());
    }
    if hotkey.mods.contains(Modifiers::SUPER) {
        parts.push("SUPER".to_owned());
    }
    let key = hotkey.key.to_string();
    parts.push(key.strip_prefix("Key").unwrap_or(&key).to_ascii_lowercase());
    Ok(parts.join("+"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn click(button: MouseButton, button_state: MouseButtonState) -> TrayIconEvent {
        TrayIconEvent::Click {
            id: "mystline".into(),
            position: tray_icon::dpi::PhysicalPosition::new(0.0, 0.0),
            rect: tray_icon::Rect::default(),
            button,
            button_state,
        }
    }

    #[test]
    fn primary_click_opens_panel_only_on_release() {
        assert!(tray_action(click(MouseButton::Left, MouseButtonState::Down)).is_none());
        assert!(matches!(
            tray_action(click(MouseButton::Left, MouseButtonState::Up)),
            Some(Event::OpenPanel)
        ));
    }

    #[test]
    fn other_tray_interactions_do_not_open_panel() {
        for button in [MouseButton::Right, MouseButton::Middle] {
            for state in [MouseButtonState::Down, MouseButtonState::Up] {
                assert!(tray_action(click(button, state)).is_none());
            }
        }
        assert!(
            tray_action(TrayIconEvent::DoubleClick {
                id: "mystline".into(),
                position: tray_icon::dpi::PhysicalPosition::new(0.0, 0.0),
                rect: tray_icon::Rect::default(),
                button: MouseButton::Left,
            })
            .is_none()
        );
        assert!(
            tray_action(TrayIconEvent::Enter {
                id: "mystline".into(),
                position: tray_icon::dpi::PhysicalPosition::new(0.0, 0.0),
                rect: tray_icon::Rect::default(),
            })
            .is_none()
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn configured_shortcuts_are_converted_for_the_wayland_portal() {
        assert_eq!(
            portal_trigger("Ctrl+Shift+Espace").unwrap(),
            "CTRL+SHIFT+space"
        );
        assert_eq!(portal_trigger("Alt+T").unwrap(), "ALT+t");
    }
}
