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
        let image = crate::brand::tray_icon();
        let icon = Icon::from_rgba(image.rgba().to_vec(), image.width(), image.height())?;
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
fn portal_app_id(identifier: &str) -> Result<String, String> {
    // KGlobalAccel réserve le suffixe .desktop aux lanceurs et non aux actions
    // dynamiques du portail. Le fichier de métadonnées ajoute déjà ce suffixe.
    let app_id = identifier.strip_suffix(".desktop").unwrap_or(identifier);
    app_id
        .parse::<ashpd::AppID>()
        .map_err(|error| error.to_string())?;
    Ok(app_id.to_owned())
}

#[cfg(target_os = "linux")]
fn portal_desktop_entry(app_id: &str, executable: &std::path::Path) -> Result<String, String> {
    // Valider avant d'utiliser l'identifiant comme nom de fichier.
    app_id
        .parse::<ashpd::AppID>()
        .map_err(|error| error.to_string())?;
    let executable = executable.to_str().ok_or("Chemin du binaire non UTF-8")?;
    // Échappements du format Desktop Entry puis de l'argument Exec entre guillemets.
    let executable = executable
        .replace('\\', "\\\\\\\\")
        .replace('"', "\\\\\"")
        .replace('`', "\\\\`")
        .replace('$', "\\\\$")
        .replace('%', "%%")
        .replace('\n', "\\n")
        .replace('\r', "\\r");
    Ok(format!(
        "[Desktop Entry]\nType=Application\nName=Mystline\nExec=\"{executable}\"\nTerminal=false\nNoDisplay=true\n"
    ))
}

#[cfg(target_os = "linux")]
fn ensure_portal_desktop_entry(
    app_id: &str,
    executable: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use std::io::Write;
    let entry = portal_desktop_entry(app_id, executable)?;
    let directories = directories::BaseDirs::new().ok_or("Répertoire utilisateur introuvable")?;
    let applications = directories.data_dir().join("applications");
    std::fs::create_dir_all(&applications)?;
    let path = applications.join(format!("{app_id}.desktop"));
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(mut file) => file.write_all(entry.as_bytes())?,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => (),
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

#[cfg(target_os = "linux")]
struct PortalBinding {
    id: String,
    trigger: String,
    capture: bool,
    generation: u64,
}

#[cfg(target_os = "linux")]
fn portal_bindings(
    shortcuts: &config::Shortcuts,
    generations: [u64; 2],
) -> Result<[PortalBinding; 2], String> {
    let binding = |value: &str, capture: bool, generation| -> Result<PortalBinding, String> {
        let trigger = portal_trigger(value)?;
        let action = if capture { "capture" } else { "panel" };
        // KDE restaure les touches d'un identifiant déjà connu et ignore alors
        // preferred_trigger. Une nouvelle combinaison doit être une nouvelle demande.
        Ok(PortalBinding {
            id: format!("{action}:{trigger}"),
            trigger,
            capture,
            generation,
        })
    };
    Ok([
        binding(&shortcuts.capture, true, generations[0])?,
        binding(&shortcuts.panel, false, generations[1])?,
    ])
}

#[cfg(target_os = "linux")]
fn granted_trigger<'a>(
    shortcuts: impl IntoIterator<Item = (&'a str, &'a str)>,
    id: &str,
) -> Option<&'a str> {
    shortcuts
        .into_iter()
        .find(|(shortcut_id, _)| *shortcut_id == id)
        .map(|(_, trigger)| trigger.trim())
        .filter(|trigger| !trigger.is_empty())
}

#[cfg(target_os = "linux")]
pub async fn portal_shortcuts(
    shortcuts: config::Shortcuts,
    generations: [u64; 2],
    app_id: String,
    mut stop: tokio::sync::oneshot::Receiver<()>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use ashpd::desktop::{
        CreateSessionOptions,
        global_shortcuts::{BindShortcutsOptions, GlobalShortcuts, NewShortcut},
    };
    use futures::StreamExt;
    let app_id = portal_app_id(&app_id)?;
    let bindings = portal_bindings(&shortcuts, generations)?;
    if !ashpd::is_sandboxed() {
        ensure_portal_desktop_entry(&app_id, &std::env::current_exe()?)?;
    }
    // Enregistrer l'identité avant tout appel de portail, sur la même connexion.
    // Une connexion dédiée permet aussi le renouvellement sans double Register.
    let connection = ashpd::zbus::Connection::session().await?;
    ashpd::register_host_app_with_connection(connection.clone(), app_id.parse()?).await?;
    let portal = GlobalShortcuts::with_connection(connection).await?;
    let session = portal
        .create_session(CreateSessionOptions::default())
        .await?;
    // Session exposes its object path through Serialize, not a public accessor.
    let session_path = serde_json::to_value(&session)?
        .as_str()
        .ok_or("Session portail invalide")?
        .to_owned();
    let result = async {
        let activated = portal.receive_activated().await?;
        let changed = portal.receive_shortcuts_changed().await?;
        let closed = session.receive_closed().await?;
        let shortcuts = bindings.each_ref().map(|binding| {
            NewShortcut::new(
                &binding.id,
                if binding.capture {
                    "Capture rapide"
                } else {
                    "Ouvrir le panneau"
                },
            )
            .preferred_trigger(Some(binding.trigger.as_str()))
        });
        let response = portal
            .bind_shortcuts(&session, &shortcuts, None, BindShortcutsOptions::default())
            .await?
            .response()?;
        let publish_status = |shortcuts: &[ashpd::desktop::global_shortcuts::Shortcut]| {
            bindings.each_ref().map(|binding| {
                let trigger = granted_trigger(
                    shortcuts
                        .iter()
                        .map(|shortcut| (shortcut.id(), shortcut.trigger_description())),
                    &binding.id,
                );
                let active = trigger.is_some();
                tracing::info!(action = %binding.id, ?trigger, active, "portal_shortcut_status");
                emit(Event::PortalStatus {
                    capture: binding.capture,
                    generation: binding.generation,
                    active,
                });
                active
            })
        };
        let mut active = publish_status(response.shortcuts());
        futures::pin_mut!(activated, changed, closed);
        loop {
            tokio::select! {
                activation = activated.next() => match activation {
                    Some(activation) if activation.session_handle().as_str() == session_path => {
                        for (index, binding) in bindings.iter().enumerate() {
                            if activation.shortcut_id() == binding.id && active[index] {
                                tracing::info!(action = %binding.id, "portal_shortcut_activated");
                                emit(Event::PortalActivated { capture: binding.capture, generation: binding.generation });
                            }
                        }
                    },
                    Some(_) => (),
                    None => break,
                },
                update = changed.next() => match update {
                    Some(update) if update.session_handle().as_str() == session_path => {
                        active = publish_status(update.shortcuts());
                    },
                    Some(_) => (),
                    None => break,
                },
                _ = closed.next() => break,
            }
        }
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    };
    let result = tokio::select! { _ = &mut stop => Ok(()), result = result => result };
    let close = session.close().await;
    for binding in bindings {
        emit(Event::PortalStatus {
            capture: binding.capture,
            generation: binding.generation,
            active: false,
        });
    }
    result?;
    close?;
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

    #[test]
    fn portal_action_without_assigned_keys_is_inactive() {
        let shortcuts = [("capture", ""), ("panel", "Ctrl+Shift+T")];
        assert_eq!(granted_trigger(shortcuts, "capture"), None);
        assert_eq!(granted_trigger(shortcuts, "missing"), None);
        assert_eq!(granted_trigger([("capture", "  ")], "capture"), None);
        assert_eq!(granted_trigger(shortcuts, "panel"), Some("Ctrl+Shift+T"));
    }

    #[test]
    fn portal_identity_has_a_hidden_desktop_entry_with_escaped_executable() {
        let app_id = portal_app_id("local.mystline.desktop").unwrap();
        assert_eq!(app_id, "local.mystline");
        assert!(!app_id.ends_with(".desktop"));
        assert_eq!(portal_app_id(&app_id).unwrap(), app_id);
        let entry =
            portal_desktop_entry(&app_id, std::path::Path::new("/tmp/My App/100%.bin")).unwrap();
        assert!(entry.contains("Name=Mystline\n"));
        assert!(entry.contains("Exec=\"/tmp/My App/100%%.bin\"\n"));
        assert!(entry.contains("NoDisplay=true\n"));
        assert!(portal_desktop_entry("../bad", std::path::Path::new("/tmp/app")).is_err());
    }

    #[test]
    fn portal_request_keeps_both_actions_when_one_combination_changes() {
        let initial = config::Shortcuts::default();
        let before = portal_bindings(&initial, [0, 0]).unwrap();
        let updated = initial.changed(true, "Alt+Shift+C").unwrap();
        let after = portal_bindings(&updated, [1, 1]).unwrap();
        assert_ne!(before[0].id, after[0].id);
        assert_eq!(before[1].id, after[1].id);
        assert_eq!(after[0].trigger, "SHIFT+ALT+c");
        assert!(after[0].capture);
        assert!(!after[1].capture);
        assert_eq!(after[0].generation, 1);
        assert_eq!(after[1].generation, 1);
        let equivalent = config::Shortcuts {
            capture: "Shift+Control+Space".into(),
            ..initial
        };
        assert_eq!(
            before[0].id,
            portal_bindings(&equivalent, [2, 2]).unwrap()[0].id
        );
    }

    #[test]
    #[ignore = "nécessite le bus D-Bus et le portail de la session graphique"]
    fn native_portal_accepts_mystline_identity_before_creating_session() {
        tauri::async_runtime::block_on(async {
            use ashpd::desktop::{CreateSessionOptions, global_shortcuts::GlobalShortcuts};
            let identifier =
                serde_json::from_str::<serde_json::Value>(include_str!("../tauri.conf.json"))
                    .unwrap()["identifier"]
                    .as_str()
                    .unwrap()
                    .to_owned();
            let identifier = portal_app_id(&identifier).unwrap();
            let connection = ashpd::zbus::Connection::session().await.unwrap();
            let executable = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("target/debug/mystline-desktop");
            ensure_portal_desktop_entry(&identifier, &executable).unwrap();
            ashpd::register_host_app_with_connection(
                connection.clone(),
                identifier.parse().unwrap(),
            )
            .await
            .unwrap();
            let portal = GlobalShortcuts::with_connection(connection).await.unwrap();
            let session = portal
                .create_session(CreateSessionOptions::default())
                .await
                .unwrap();
            session.close().await.unwrap();
        });
    }

    #[test]
    #[ignore = "nécessite KDE et les deux raccourcis Mystline déjà autorisés"]
    fn native_kde_restores_granted_shortcuts_in_new_sessions() {
        tauri::async_runtime::block_on(async {
            use ashpd::desktop::{
                CreateSessionOptions,
                global_shortcuts::{GlobalShortcuts, ListShortcutsOptions},
            };
            let identifier =
                serde_json::from_str::<serde_json::Value>(include_str!("../tauri.conf.json"))
                    .unwrap()["identifier"]
                    .as_str()
                    .unwrap()
                    .to_owned();
            let app_id = portal_app_id(&identifier).unwrap();
            let directory = config::directory().unwrap();
            let config = config::load_or_create(&directory).unwrap();
            let bindings = portal_bindings(&config.shortcuts, [0, 0]).unwrap();
            // Deux connexions neuves reproduisent deux lancements indépendants.
            // ListShortcuts n'affiche aucun dialogue et ne réattribue aucune touche.
            for _ in 0..2 {
                let connection = ashpd::zbus::Connection::session().await.unwrap();
                ashpd::register_host_app_with_connection(
                    connection.clone(),
                    app_id.parse().unwrap(),
                )
                .await
                .unwrap();
                let portal = GlobalShortcuts::with_connection(connection).await.unwrap();
                let session = portal
                    .create_session(CreateSessionOptions::default())
                    .await
                    .unwrap();
                let response = portal
                    .list_shortcuts(&session, ListShortcutsOptions::default())
                    .await
                    .unwrap()
                    .response()
                    .unwrap();
                for binding in &bindings {
                    assert!(
                        granted_trigger(
                            response
                                .shortcuts()
                                .iter()
                                .map(|shortcut| (shortcut.id(), shortcut.trigger_description())),
                            &binding.id,
                        )
                        .is_some(),
                        "KDE doit restaurer la combinaison de {}",
                        binding.id
                    );
                }
                session.close().await.unwrap();
            }
        });
    }
}
