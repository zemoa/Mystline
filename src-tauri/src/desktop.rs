use crate::surfaces::{self, Surface, Surfaces};
use crate::tasks::{self, TaskView, ViewRequest};
use chrono::Local;
use mystline::{
    config::{self, Config},
    presentation::panel::next_day_delay,
    runtime::{
        coordinator::{Coordinator, Creation, Operation},
        task_store::{JobResult, SourceRevision},
    },
};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use std::{
    collections::HashMap,
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
    PortalStatus {
        capture: bool,
        generation: u64,
        active: bool,
    },
    PortalActivated {
        capture: bool,
        generation: u64,
    },
    RepositoryFinished(JobResult, Option<String>),
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
    runtime: Coordinator<String>,
    shortcuts: (bool, bool),
    repository_error: Option<String>,
    quit_pending: bool,
    directory: PathBuf,
    version: u64,
    responses: HashMap<String, tokio::sync::oneshot::Sender<Result<(), String>>>,
    shortcut_generations: [u64; 2],
    #[cfg(target_os = "linux")]
    portal_controls: [Option<PortalControl>; 2],
    is_probe: bool,
    _watcher: RecommendedWatcher,
}

#[cfg(target_os = "linux")]
struct PortalControl {
    stop: tokio::sync::oneshot::Sender<()>,
    closed: tokio::sync::oneshot::Receiver<()>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopStatus {
    version: u64,
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
        version: resident.version,
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
    let _ = app.emit("task-changed", ());
}

#[tauri::command]
pub fn task_view(app: AppHandle, request: ViewRequest) -> TaskView {
    let state = app.state::<Mutex<Resident>>();
    let resident = state.lock().expect("état résident");
    tasks::project(
        &resident.runtime.tasks.snapshot,
        resident.runtime.tasks.source_revision(),
        resident.runtime.tasks.is_busy(),
        &request,
        Local::now().date_naive(),
        resident.version,
    )
}

async fn completion(
    receiver: tokio::sync::oneshot::Receiver<Result<(), String>>,
) -> Result<(), String> {
    receiver
        .await
        .map_err(|_| "La confirmation de l'enregistrement est indisponible.".to_owned())?
}

#[tauri::command]
pub async fn task_create(
    window: tauri::WebviewWindow,
    draft: String,
    session: u64,
    activation: u64,
) -> Result<(), String> {
    if !matches!(window.label(), "capture" | "panel") {
        return Err("Surface de saisie inconnue.".into());
    }
    let input = mystline::presentation::input::parse_input(&draft, Local::now().date_naive())
        .map_err(|error| error.to_string())?;
    let app = window.app_handle().clone();
    let origin = format!("{}:{session}:{activation}", window.label());
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let operation = {
        let state = app.state::<Mutex<Resident>>();
        let mut state = state.lock().map_err(|error| error.to_string())?;
        if state.responses.contains_key(&origin) {
            return Err("Cette saisie est déjà en cours d'enregistrement.".into());
        }
        let operation = state.runtime.create(Creation {
            origin: origin.clone(),
            draft,
            input,
        })?;
        state.responses.insert(origin, sender);
        state.version += 1;
        operation
    };
    publish(&app);
    if let Some(operation) = operation {
        perform(operation, None);
    }
    completion(receiver).await
}

async fn write_command(
    app: AppHandle,
    source: SourceRevision,
    command: mystline::application::TaskCommand,
) -> Result<(), String> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let (operation, origin) = {
        let state = app.state::<Mutex<Resident>>();
        let mut state = state.lock().map_err(|error| error.to_string())?;
        let operation = state.runtime.write(source, command)?;
        let origin = format!("write:{}", state.version);
        state.responses.insert(origin.clone(), sender);
        state.version += 1;
        (operation, origin)
    };
    publish(&app);
    perform(operation, Some(origin));
    completion(receiver).await
}

#[tauri::command]
pub async fn task_update(
    app: AppHandle,
    source: SourceRevision,
    position: usize,
    draft: String,
) -> Result<(), String> {
    write_command(
        app,
        source,
        tasks::update(position, &draft, Local::now().date_naive())?,
    )
    .await
}

#[tauri::command]
pub async fn task_toggle(
    app: AppHandle,
    source: SourceRevision,
    position: usize,
    request: ViewRequest,
) -> Result<(), String> {
    let command = {
        let state = app.state::<Mutex<Resident>>();
        let state = state.lock().map_err(|error| error.to_string())?;
        if source != state.runtime.tasks.source_revision() {
            return Err("La liste ou le fichier source a changé.".into());
        }
        tasks::toggle(
            &state.runtime.tasks.snapshot,
            &request,
            position,
            Local::now().date_naive(),
        )?
    };
    write_command(app, source, command).await
}

#[tauri::command]
pub async fn task_reorder(
    app: AppHandle,
    source: SourceRevision,
    position: usize,
    down: bool,
    request: ViewRequest,
) -> Result<Option<usize>, String> {
    let command = {
        let state = app.state::<Mutex<Resident>>();
        let state = state.lock().map_err(|error| error.to_string())?;
        if source != state.runtime.tasks.source_revision() {
            return Err("La liste ou le fichier source a changé.".into());
        }
        tasks::reorder(
            &state.runtime.tasks.snapshot,
            &request,
            position,
            down,
            Local::now().date_naive(),
        )
    };
    if let Some(command) = command {
        let target = match &command {
            mystline::application::TaskCommand::Reorder { to, .. } => *to,
            _ => unreachable!(),
        };
        write_command(app, source, command).await?;
        Ok(Some(target))
    } else {
        Ok(None)
    }
}

pub(crate) fn watch_event(event: &notify::Event, target: &Path) -> bool {
    use notify::event::{AccessKind, AccessMode};
    // Les lectures du repository produisent aussi des événements Access de lecture :
    // les recharger créerait une boucle alimentée par nos propres lectures.
    event.need_rescan()
        || (matches!(
            event.kind,
            notify::EventKind::Create(_)
                | notify::EventKind::Modify(_)
                | notify::EventKind::Remove(_)
                | notify::EventKind::Access(AccessKind::Close(AccessMode::Write))
        ) && event.paths.iter().any(|path| path == target))
}

fn watch(path: &Path) -> notify::Result<RecommendedWatcher> {
    let target = std::fs::canonicalize(path)?;
    let parent = target.parent().expect("fichier absolu").to_path_buf();
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if event
            .as_ref()
            .is_ok_and(|event| watch_event(event, &target))
            && !FILE_CHANGE_PENDING.swap(true, Ordering::AcqRel)
        {
            emit(Event::FileChanged);
        }
    })?;
    watcher.watch(&parent, RecursiveMode::NonRecursive)?;
    Ok(watcher)
}

fn perform(operation: Operation<String>, origin: Option<String>) {
    tauri::async_runtime::spawn(async move {
        let origin = operation
            .creation
            .map(|creation| creation.origin)
            .or(origin);
        emit(Event::RepositoryFinished(
            operation.job.perform().await,
            origin,
        ));
    });
}

async fn event_loop(app: AppHandle, mut receiver: tokio::sync::mpsc::UnboundedReceiver<Event>) {
    while let Some(event) = receiver.recv().await {
        match event {
            Event::FileChanged => {
                FILE_CHANGE_PENDING.store(false, Ordering::Release);
                let operation = {
                    let state = app.state::<Mutex<Resident>>();
                    let mut state = state.lock().expect("état résident");
                    let operation = state.runtime.reload();
                    state.version += 1;
                    operation
                };
                if let Some(operation) = operation {
                    publish(&app);
                    perform(operation, None);
                }
            }
            Event::RepositoryFinished(result, origin) => {
                let (operation, quit) = {
                    let state = app.state::<Mutex<Resident>>();
                    let mut state = state.lock().expect("état résident");
                    if !state.runtime.finish(&result) {
                        continue;
                    }
                    state.repository_error = result.error;
                    if let Some(sender) = origin.and_then(|origin| state.responses.remove(&origin))
                    {
                        let _ = sender.send(state.repository_error.clone().map_or(Ok(()), Err));
                    }
                    let operation = state.runtime.next_operation().unwrap_or_else(|error| {
                        state.repository_error = Some(error);
                        None
                    });
                    let quit = state.quit_pending && operation.is_none();
                    state.version += 1;
                    (operation, quit)
                };
                publish(&app);
                if let Some(operation) = operation {
                    perform(operation, None);
                }
                if quit {
                    app.exit(0);
                }
            }
            Event::PortalStatus {
                capture,
                generation,
                active,
            } => {
                let index = usize::from(!capture);
                let accepted = {
                    let state = app.state::<Mutex<Resident>>();
                    let mut state = state.lock().expect("état résident");
                    if state.shortcut_generations[index] != generation {
                        false
                    } else {
                        if capture {
                            state.shortcuts.0 = active;
                        } else {
                            state.shortcuts.1 = active;
                        }
                        state.version += 1;
                        true
                    }
                };
                if accepted {
                    publish(&app);
                }
            }
            Event::PortalActivated {
                capture,
                generation,
            } => {
                let index = usize::from(!capture);
                let accepted = app
                    .state::<Mutex<Resident>>()
                    .lock()
                    .expect("état résident")
                    .shortcut_generations[index]
                    == generation;
                if accepted {
                    emit(if capture {
                        Event::Capture
                    } else {
                        Event::Panel
                    });
                }
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
    (
        register_shortcut(app, &shortcuts.capture, true, 0),
        register_shortcut(app, &shortcuts.panel, false, 0),
    )
}

fn register_shortcut(app: &AppHandle, value: &str, capture: bool, generation: u64) -> bool {
    config::parse_shortcut(value).ok().is_some_and(|key| {
        app.global_shortcut()
            .on_shortcut(key, move |_, _, shortcut| {
                if shortcut.state == ShortcutState::Pressed {
                    emit(Event::PortalActivated {
                        capture,
                        generation,
                    });
                }
            })
            .is_ok()
    })
}

#[cfg(target_os = "linux")]
fn start_portal(
    value: String,
    capture: bool,
    generation: u64,
    previous: Option<PortalControl>,
) -> PortalControl {
    let (stop, receiver) = tokio::sync::oneshot::channel();
    let (closed, completion) = tokio::sync::oneshot::channel();
    tauri::async_runtime::spawn(async move {
        if let Some(previous) = previous {
            let _ = previous.stop.send(());
            let _ = previous.closed.await;
        }
        if let Err(error) =
            crate::linux::portal_shortcut(value, capture, generation, receiver).await
        {
            tracing::warn!(%error, "portal_shortcut_unavailable");
            emit(Event::PortalStatus {
                capture,
                generation,
                active: false,
            });
        }
        let _ = closed.send(());
    });
    PortalControl {
        stop,
        closed: completion,
    }
}

#[tauri::command]
pub async fn settings_source(app: AppHandle, path: String, create: bool) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("Indiquer le chemin du fichier de tâches.".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let path = PathBuf::from(path);
        let path = if path.is_absolute() {
            path
        } else {
            std::env::current_dir()
                .map_err(|error| error.to_string())?
                .join(path)
        };
        let state = app.state::<Mutex<Resident>>();
        {
            let state = state.lock().map_err(|error| error.to_string())?;
            if !state.runtime.can_replace_source() {
                return Err(
                    "Attendre la fin des enregistrements avant de changer de fichier.".into(),
                );
            }
        }
        // Préparer le fichier hors du verrou résident pour garder les autres surfaces réactives.
        mystline::repository::TaskRepository::open(path.clone(), create)
            .map_err(|error| error.to_string())?;
        let path = std::fs::canonicalize(path).map_err(|error| error.to_string())?;
        let repository = mystline::repository::TaskRepository::open(path.clone(), false)
            .map_err(|error| error.to_string())?;
        let watcher = watch(&path).map_err(|error| error.to_string())?;
        let mut state = state.lock().map_err(|error| error.to_string())?;
        if !state.runtime.can_replace_source() {
            return Err("Attendre la fin des enregistrements avant de changer de fichier.".into());
        }
        let mut updated = state.config.clone();
        updated.tasks_file = path;
        config::save(&state.directory, &updated).map_err(|error| error.to_string())?;
        state.runtime.replace_source(repository)?;
        state._watcher = watcher;
        state.config = updated;
        state.repository_error = None;
        state.version += 1;
        drop(state);
        publish(&app);
        emit(Event::FileChanged);
        Ok(())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn settings_choose(app: AppHandle, create: bool) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let dialog = app.dialog().file().add_filter("Fichier Markdown", &["md"]);
    let callback = move |path: Option<tauri_plugin_dialog::FilePath>| {
        let result = path
            .map(|path| {
                path.into_path()
                    .map(|path| path.display().to_string())
                    .map_err(|error| error.to_string())
            })
            .transpose();
        let _ = sender.send(result);
    };
    if create {
        dialog.set_file_name("tasks.md").save_file(callback);
    } else {
        dialog.pick_file(callback);
    }
    receiver.await.map_err(|error| error.to_string())?
}

#[tauri::command]
pub fn settings_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    let state = app.state::<Mutex<Resident>>();
    let mut state = state.lock().map_err(|error| error.to_string())?;
    let previous = state.config.autostart;
    if !state.is_probe {
        (if enabled {
            app.autolaunch().enable()
        } else {
            app.autolaunch().disable()
        })
        .map_err(|error| error.to_string())?;
    }
    let mut updated = state.config.clone();
    updated.autostart = enabled;
    if let Err(error) = config::save(&state.directory, &updated) {
        if !state.is_probe {
            let _ = if previous {
                app.autolaunch().enable()
            } else {
                app.autolaunch().disable()
            };
        }
        return Err(error.to_string());
    }
    state.config = updated;
    state.version += 1;
    drop(state);
    publish(&app);
    Ok(())
}

#[tauri::command]
pub fn settings_shortcut(app: AppHandle, capture: bool, value: String) -> Result<(), String> {
    let state = app.state::<Mutex<Resident>>();
    let mut state = state.lock().map_err(|error| error.to_string())?;
    let mut updated = state.config.clone();
    updated.shortcuts = updated.shortcuts.changed(capture, &value)?;
    config::save(&state.directory, &updated).map_err(|error| error.to_string())?;
    let previous = if capture {
        state.config.shortcuts.capture.clone()
    } else {
        state.config.shortcuts.panel.clone()
    };
    let requested = if capture {
        &updated.shortcuts.capture
    } else {
        &updated.shortcuts.panel
    };
    let index = usize::from(!capture);
    state.shortcut_generations[index] += 1;
    let generation = state.shortcut_generations[index];
    let active = if state.is_probe {
        false
    } else if wayland() {
        #[cfg(target_os = "linux")]
        {
            let previous = state.portal_controls[index].take();
            state.portal_controls[index] = Some(start_portal(
                requested.clone(),
                capture,
                generation,
                previous,
            ));
        }
        false
    } else {
        if let Ok(key) = config::parse_shortcut(&previous) {
            let _ = app.global_shortcut().unregister(key);
        }
        register_shortcut(&app, requested, capture, generation)
    };
    if capture {
        state.shortcuts.0 = active;
    } else {
        state.shortcuts.1 = active;
    }
    state.config = updated;
    state.version += 1;
    drop(state);
    publish(&app);
    Ok(())
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

#[cfg(test)]
#[path = "desktop_tests.rs"]
mod tests;

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
        .plugin(tauri_plugin_dialog::init())
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
        task_view,
        task_create,
        task_update,
        task_toggle,
        task_reorder,
        settings_source,
        settings_autostart,
        settings_shortcut,
        settings_choose,
        surfaces::surface_ready,
        surfaces::open_surface,
        surfaces::close_surface,
        surfaces::close_current_surface,
        crate::probe::probe_capture_state,
        crate::probe::probe_dom_state,
        surfaces::capture_layout
    ]);
    #[cfg(not(debug_assertions))]
    let builder = builder.invoke_handler(tauri::generate_handler![
        desktop_status,
        task_view,
        task_create,
        task_update,
        task_toggle,
        task_reorder,
        settings_source,
        settings_autostart,
        settings_shortcut,
        settings_choose,
        surfaces::surface_ready,
        surfaces::open_surface,
        surfaces::close_surface,
        surfaces::close_current_surface,
        surfaces::capture_layout
    ]);
    let app = builder
        .setup(move |app| {
            let mut config = config::load_or_create(&directory)?;
            if config.tasks_file.is_relative() {
                config.tasks_file = std::env::current_dir()?.join(&config.tasks_file);
            }
            mystline::repository::TaskRepository::open(config.tasks_file.clone(), true)?;
            config.tasks_file = std::fs::canonicalize(&config.tasks_file)?;
            let runtime = Coordinator::open(config.tasks_file.clone(), false)?;
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
            let portal_controls = if wayland() && !is_probe {
                [
                    Some(start_portal(
                        config.shortcuts.capture.clone(),
                        true,
                        0,
                        None,
                    )),
                    Some(start_portal(config.shortcuts.panel.clone(), false, 0, None)),
                ]
            } else {
                [None, None]
            };
            app.manage(Mutex::new(Resident {
                config,
                runtime,
                shortcuts,
                repository_error,
                quit_pending: false,
                directory: directory.clone(),
                version: 0,
                responses: HashMap::new(),
                shortcut_generations: [0, 0],
                #[cfg(target_os = "linux")]
                portal_controls,
                is_probe,
                _watcher: watcher,
            }));
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(event_loop(handle.clone(), receiver));
            // Ferme l'intervalle entre lecture initiale et installation du watcher.
            emit(Event::FileChanged);
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(next_day_delay(Local::now())).await;
                    handle
                        .state::<Mutex<Resident>>()
                        .lock()
                        .expect("état résident")
                        .version += 1;
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
