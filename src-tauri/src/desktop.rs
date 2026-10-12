use crate::surfaces::{self, Surface, Surfaces};
use chrono::Local;
use mystline::{
    config::{self, Config},
    presentation::panel::next_day_delay,
    runtime::{
        coordinator::{Coordinator, Operation},
        task_store::JobResult,
    },
};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

#[derive(Debug, Clone)]
pub enum Event {
    Capture,
    Panel,
    OpenPanel,
    Settings,
    Quit,
    FileChanged,
    ShortcutStatus(bool, bool),
    RepositoryFinished(JobResult),
}
static EVENTS: OnceLock<tokio::sync::mpsc::UnboundedSender<Event>> = OnceLock::new();
static FILE_CHANGE_PENDING: AtomicBool = AtomicBool::new(false);
pub fn emit(event: Event) {
    if let Some(sender) = EVENTS.get() {
        let _ = sender.send(event);
    }
}

struct Resident {
    config: Config,
    runtime: Coordinator<u64>,
    shortcuts: (bool, bool),
    repository_error: Option<String>,
    quit_pending: bool,
    _watcher: RecommendedWatcher,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopStatus {
    tasks_file: String,
    pub(crate) task_count: usize,
    revision: u64,
    today: String,
    autostart: bool,
    capture_shortcut: String,
    panel_shortcut: String,
    capture_active: bool,
    panel_active: bool,
    pub(crate) repository_error: Option<String>,
}

#[tauri::command]
pub fn desktop_status(app: AppHandle) -> DesktopStatus {
    let state = app.state::<Mutex<Resident>>();
    let resident = state.lock().expect("état résident");
    DesktopStatus {
        tasks_file: resident.config.tasks_file.display().to_string(),
        task_count: resident.runtime.tasks.snapshot.tasks.len(),
        revision: resident.runtime.tasks.snapshot.revision,
        today: Local::now().date_naive().to_string(),
        autostart: resident.config.autostart,
        capture_shortcut: resident.config.shortcuts.capture.clone(),
        panel_shortcut: resident.config.shortcuts.panel.clone(),
        capture_active: resident.shortcuts.0,
        panel_active: resident.shortcuts.1,
        repository_error: resident.repository_error.clone(),
    }
}

fn publish(app: &AppHandle) {
    let _ = app.emit("desktop-status", desktop_status(app.clone()));
}

fn watch(path: &Path) -> notify::Result<RecommendedWatcher> {
    let target = path.to_path_buf();
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if event
            .as_ref()
            .is_ok_and(|event| event.paths.iter().any(|path| path == &target))
            && !FILE_CHANGE_PENDING.swap(true, Ordering::AcqRel)
        {
            emit(Event::FileChanged);
        }
    })?;
    watcher.watch(
        path.parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new(".")),
        RecursiveMode::NonRecursive,
    )?;
    Ok(watcher)
}

fn perform(operation: Operation<u64>) {
    tauri::async_runtime::spawn(async move {
        emit(Event::RepositoryFinished(operation.job.perform().await));
    });
}

async fn event_loop(app: AppHandle, mut receiver: tokio::sync::mpsc::UnboundedReceiver<Event>) {
    while let Some(event) = receiver.recv().await {
        match event {
            Event::FileChanged => {
                FILE_CHANGE_PENDING.store(false, Ordering::Release);
                let operation = app
                    .state::<Mutex<Resident>>()
                    .lock()
                    .expect("état résident")
                    .runtime
                    .reload();
                if let Some(operation) = operation {
                    perform(operation);
                }
            }
            Event::RepositoryFinished(result) => {
                let (operation, quit) = {
                    let state = app.state::<Mutex<Resident>>();
                    let mut state = state.lock().expect("état résident");
                    if !state.runtime.finish(&result) {
                        continue;
                    }
                    state.repository_error = result.error;
                    let operation = state.runtime.next_operation().unwrap_or_else(|error| {
                        state.repository_error = Some(error);
                        None
                    });
                    let quit = state.quit_pending && operation.is_none();
                    (operation, quit)
                };
                publish(&app);
                if let Some(operation) = operation {
                    perform(operation);
                }
                if quit {
                    app.exit(0);
                }
            }
            Event::ShortcutStatus(capture, panel) => {
                app.state::<Mutex<Resident>>()
                    .lock()
                    .expect("état résident")
                    .shortcuts = (capture, panel);
                publish(&app);
            }
            Event::Quit => {
                let busy = {
                    let state = app.state::<Mutex<Resident>>();
                    let mut state = state.lock().expect("état résident");
                    state.quit_pending = true;
                    state.runtime.tasks.is_busy()
                };
                if !busy {
                    app.exit(0);
                }
            }
            event => {
                let handle = app.clone();
                let _ = app.run_on_main_thread(move || {
                    let result = match event {
                        Event::Capture => surfaces::open(&handle, Surface::Capture),
                        Event::Panel => surfaces::toggle_panel(&handle),
                        Event::OpenPanel => surfaces::open(&handle, Surface::Panel),
                        Event::Settings => surfaces::open(&handle, Surface::Settings),
                        _ => Ok(()),
                    };
                    if let Err(error) = result {
                        tracing::error!(%error, "desktop_action_failed");
                    }
                });
            }
        }
    }
}

fn register_shortcuts(app: &AppHandle, shortcuts: &config::Shortcuts) -> (bool, bool) {
    let register = |value: &str, event: Event| {
        config::parse_shortcut(value).ok().is_some_and(|key| {
            app.global_shortcut()
                .on_shortcut(key, move |_, _, shortcut| {
                    if shortcut.state == ShortcutState::Pressed {
                        emit(event.clone());
                    }
                })
                .is_ok()
        })
    };
    (
        register(&shortcuts.capture, Event::Capture),
        register(&shortcuts.panel, Event::Panel),
    )
}

fn wayland() -> bool {
    cfg!(target_os = "linux")
        && (std::env::var("XDG_SESSION_TYPE")
            .is_ok_and(|value| value.eq_ignore_ascii_case("wayland"))
            || (std::env::var_os("WAYLAND_DISPLAY").is_some()
                && std::env::var_os("DISPLAY").is_none()))
}

#[cfg(target_os = "linux")]
thread_local! { static TRAY: std::cell::RefCell<Option<crate::linux::Tray>> = const { std::cell::RefCell::new(None) }; }

#[cfg(windows)]
fn install_tray(app: &AppHandle) -> tauri::Result<()> {
    use tauri::{
        menu::{Menu, MenuItem},
        tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    };
    let menu = Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "capture", "Capture rapide", true, None::<&str>)?,
            &MenuItem::with_id(
                app,
                "panel",
                "Ouvrir / masquer le panneau",
                true,
                None::<&str>,
            )?,
            &MenuItem::with_id(app, "settings", "Paramètres", true, None::<&str>)?,
            &MenuItem::with_id(app, "quit", "Quitter", true, None::<&str>)?,
        ],
    )?;
    let mut pixels = vec![0; 32 * 32 * 4];
    for y in 6..26 {
        for x in 6..26 {
            pixels[(y * 32 + x) * 4..(y * 32 + x) * 4 + 4].copy_from_slice(&[87, 111, 233, 255]);
        }
    }
    TrayIconBuilder::with_id("mystline")
        .icon(tauri::image::Image::new_owned(pixels, 32, 32))
        .tooltip("Mystline")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|_, event| {
            if let Some(event) = menu_action(event.id().as_ref()) {
                emit(event);
            }
        })
        .on_tray_icon_event(|_, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                emit(Event::OpenPanel);
            }
        })
        .build(app)?;
    Ok(())
}

#[cfg(windows)]
fn menu_action(id: &str) -> Option<Event> {
    Some(match id {
        "capture" => Event::Capture,
        "panel" => Event::Panel,
        "settings" => Event::Settings,
        "quit" => Event::Quit,
        _ => return None,
    })
}

fn init_logging(dir: &Path) -> tracing_appender::non_blocking::WorkerGuard {
    let logs = dir.join("logs");
    let _ = std::fs::create_dir_all(&logs);
    let appender = tracing_appender::rolling::daily(logs, "mystline.log");
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let _ = tracing::subscriber::set_global_default(
        tracing_subscriber::fmt::Subscriber::builder()
            .with_ansi(false)
            .with_target(false)
            .with_writer(writer)
            .finish(),
    );
    guard
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(debug_assertions)]
    let probe_directory = crate::probe::directory();
    #[cfg(not(debug_assertions))]
    let probe_directory: Option<PathBuf> = None;
    let dir = match &probe_directory {
        Some(directory) => directory.clone(),
        None => config::directory()?,
    };
    std::fs::create_dir_all(&dir)?;
    let _log_guard = init_logging(&dir);
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    let _ = EVENTS.set(sender);
    let directory: PathBuf = dir;
    let is_probe = probe_directory.is_some();
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|_, _, _| {}))
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .app_name("mystline")
                .build(),
        );
    if !wayland() {
        builder = builder.plugin(tauri_plugin_global_shortcut::Builder::new().build());
    }
    let builder = builder.manage(Mutex::new(Surfaces::default()));
    #[cfg(debug_assertions)]
    let builder = builder.invoke_handler(tauri::generate_handler![
        desktop_status,
        surfaces::surface_ready,
        surfaces::open_surface,
        surfaces::close_surface,
        crate::probe::probe_capture_state
    ]);
    #[cfg(not(debug_assertions))]
    let builder = builder.invoke_handler(tauri::generate_handler![
        desktop_status,
        surfaces::surface_ready,
        surfaces::open_surface,
        surfaces::close_surface
    ]);
    let app = builder
        .setup(move |app| {
            let config = config::load_or_create(&directory)?;
            let runtime = Coordinator::open(config.tasks_file.clone(), true)?;
            let watcher = watch(&config.tasks_file)?;
            let repository_error = None;
            let shortcuts = if wayland() || is_probe {
                (false, false)
            } else {
                register_shortcuts(app.handle(), &config.shortcuts)
            };
            if !is_probe {
                let autostart = if config.autostart {
                    app.autolaunch().enable()
                } else {
                    app.autolaunch().disable()
                };
                if let Err(error) = autostart {
                    tracing::warn!(%error, "autostart_unavailable");
                }
            }
            #[cfg(target_os = "linux")]
            TRAY.with(|tray| -> Result<(), Box<dyn std::error::Error>> {
                *tray.borrow_mut() = Some(crate::linux::Tray::new()?);
                Ok(())
            })?;
            #[cfg(windows)]
            install_tray(app.handle())?;
            #[cfg(target_os = "linux")]
            if wayland() && !is_probe {
                let shortcuts = config.shortcuts.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(error) = crate::linux::portal_shortcuts(shortcuts).await {
                        tracing::warn!(%error, "portal_shortcuts_unavailable");
                        emit(Event::ShortcutStatus(false, false));
                    }
                });
            }
            app.manage(Mutex::new(Resident {
                config,
                runtime,
                shortcuts,
                repository_error,
                quit_pending: false,
                _watcher: watcher,
            }));
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(event_loop(handle.clone(), receiver));
            // Ferme l'intervalle entre lecture initiale et installation du watcher.
            emit(Event::FileChanged);
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(next_day_delay(Local::now())).await;
                    publish(&handle);
                }
            });
            tracing::info!("application_started_without_windows");
            #[cfg(debug_assertions)]
            if let Some(directory) = probe_directory {
                crate::probe::start(app.handle().clone(), directory);
            }
            Ok(())
        })
        .build(tauri::generate_context!())?;
    app.run(|_, event| {
        if let tauri::RunEvent::ExitRequested {
            ref api,
            code: None,
            ..
        } = event
        {
            api.prevent_exit();
        }
        #[cfg(target_os = "linux")]
        if matches!(event, tauri::RunEvent::Exit) {
            TRAY.with(|tray| {
                tray.borrow_mut().take();
            });
        }
    });
    Ok(())
}
