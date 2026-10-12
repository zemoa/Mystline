//! Vérification graphique explicite en debug, avec données temporaires.
//! Aucun enregistrement de raccourcis ni changement de l'autostart pendant la sonde.
use crate::{
    desktop,
    surfaces::{self, Surface, Surfaces},
};
use serde::Serialize;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager};

pub fn directory() -> Option<PathBuf> {
    std::env::var_os("MYSTLINE_PROBE_DIR").map(PathBuf::from)
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum WindowMode {
    Destroy,
    Hide,
}

impl WindowMode {
    fn requested() -> Self {
        if std::env::args().any(|argument| argument == "--probe-destroy-windows") {
            Self::Destroy
        } else {
            Self::Hide
        }
    }
}

#[derive(Serialize)]
struct Memory {
    processes: usize,
    rss_kib: u64,
    pss_kib: u64,
}

#[cfg(target_os = "linux")]
fn memory() -> Memory {
    let mut pids = vec![std::process::id()];
    let mut index = 0;
    while index < pids.len() {
        let pid = pids[index];
        if let Ok(tasks) = std::fs::read_dir(format!("/proc/{pid}/task")) {
            for task in tasks.flatten() {
                if let Ok(children) = std::fs::read_to_string(task.path().join("children")) {
                    for child in children
                        .split_whitespace()
                        .filter_map(|value| value.parse::<u32>().ok())
                    {
                        if !pids.contains(&child) {
                            pids.push(child);
                        }
                    }
                }
            }
        }
        index += 1;
    }
    let mut result = Memory {
        processes: pids.len(),
        rss_kib: 0,
        pss_kib: 0,
    };
    for pid in pids {
        if let Ok(smaps) = std::fs::read_to_string(format!("/proc/{pid}/smaps_rollup")) {
            for line in smaps.lines() {
                let mut fields = line.split_whitespace();
                let key = fields.next();
                let value = fields
                    .next()
                    .and_then(|value| value.parse::<u64>().ok())
                    .unwrap_or(0);
                match key {
                    Some("Rss:") => result.rss_kib += value,
                    Some("Pss:") => result.pss_kib += value,
                    _ => (),
                }
            }
        }
    }
    result
}

#[cfg(not(target_os = "linux"))]
fn memory() -> Memory {
    Memory {
        processes: 0,
        rss_kib: 0,
        pss_kib: 0,
    }
}

async fn on_main<T: Send + 'static>(
    app: &AppHandle,
    action: impl FnOnce(&AppHandle) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let handle = app.clone();
    app.run_on_main_thread(move || {
        let _ = sender.send(action(&handle));
    })
    .map_err(|error| error.to_string())?;
    receiver.await.map_err(|error| error.to_string())?
}

async fn wait_until(condition: impl Fn() -> bool) -> Result<(), String> {
    let start = Instant::now();
    while !condition() {
        if start.elapsed() > Duration::from_secs(8) {
            return Err("Délai de vérification dépassé".into());
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    Ok(())
}

fn visible_windows(app: &AppHandle) -> Result<usize, String> {
    app.webview_windows().values().try_fold(0, |count, window| {
        window
            .is_visible()
            .map(|visible| count + usize::from(visible))
            .map_err(|error| error.to_string())
    })
}

async fn open(app: &AppHandle, surface: Surface) -> Result<(f64, bool), String> {
    let start = Instant::now();
    on_main(app, move |app| {
        surfaces::open(app, surface).map_err(|error| error.to_string())?;
        surfaces::open(app, surface).map_err(|error| error.to_string())
    })
    .await?;
    wait_until(|| {
        app.get_webview_window(surface.label())
            .is_some_and(|window| window.is_visible().unwrap_or(false))
    })
    .await?;
    wait_until(|| {
        app.state::<Mutex<Surfaces>>()
            .lock()
            .unwrap()
            .ready(surface)
    })
    .await?;
    let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
    tokio::time::sleep(Duration::from_millis(100)).await;
    let focused = app
        .get_webview_window(surface.label())
        .ok_or("Fenêtre absente")?
        .is_focused()
        .map_err(|error| error.to_string())?;
    Ok((duration_ms, focused))
}

async fn close(app: &AppHandle, surface: Surface, mode: WindowMode) -> Result<(), String> {
    on_main(app, move |app| {
        if mode == WindowMode::Destroy {
            app.get_webview_window(surface.label())
                .ok_or("Fenêtre absente")?
                .destroy()
                .map_err(|error| error.to_string())
        } else {
            surfaces::close(app, surface).map_err(|error| error.to_string())
        }
    })
    .await?;
    wait_until(|| match mode {
        WindowMode::Destroy => app.get_webview_window(surface.label()).is_none(),
        WindowMode::Hide => app
            .get_webview_window(surface.label())
            .is_some_and(|window| !window.is_visible().unwrap_or(true)),
    })
    .await
}

#[derive(Serialize)]
struct Report {
    profile: &'static str,
    window_mode: WindowMode,
    session_type: Option<String>,
    startup_without_windows: bool,
    capture_ready_ms: Vec<f64>,
    native_focus_observed: Vec<bool>,
    resident_memory_before_windows: Memory,
    resident_memory_after_windows: Memory,
    resident_memory_after_10_seconds: Memory,
    second_instance_exit_success: bool,
    watcher_valid_reload: bool,
    watcher_invalid_preserved: bool,
    all_windows_destroyed: bool,
    visible_windows_after_close: usize,
    retained_native_windows_after_close: usize,
    capture_drafts_reset: bool,
    capture_input_focused: bool,
    repeated_open_keeps_draft: bool,
    capture_webview_sessions: Vec<u64>,
    panel_toggle_reopens_same_window: bool,
    native_close_masks_settings: bool,
    capture_escape_masks_window: bool,
}

struct CaptureState {
    value: String,
    focused: bool,
    session: u64,
}
type ObservationSenders = Mutex<HashMap<u64, tokio::sync::oneshot::Sender<CaptureState>>>;
static OBSERVATIONS: OnceLock<ObservationSenders> = OnceLock::new();
static OBSERVATION_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[tauri::command]
pub fn probe_capture_state(
    window: tauri::WebviewWindow,
    token: u64,
    value: String,
    focused: bool,
    session: u64,
) -> Result<(), String> {
    if directory().is_none() || window.label() != "capture" {
        return Err("Observation réservée à la sonde".into());
    }
    let sender = OBSERVATIONS
        .get_or_init(Mutex::default)
        .lock()
        .map_err(|error| error.to_string())?
        .remove(&token)
        .ok_or("Observation expirée")?;
    sender
        .send(CaptureState {
            value,
            focused,
            session,
        })
        .map_err(|_| "Observation annulée".into())
}

async fn capture_state(app: &AppHandle) -> Result<CaptureState, String> {
    let token = OBSERVATION_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let (sender, receiver) = tokio::sync::oneshot::channel();
    OBSERVATIONS
        .get_or_init(Mutex::default)
        .lock()
        .unwrap()
        .insert(token, sender);
    let window = app.get_webview_window("capture").ok_or("Capture absente")?;
    window.eval(format!("(() => {{ const input = document.querySelector('#capture'); window.__TAURI_INTERNALS__.invoke('probe_capture_state', {{ token: {token}, value: input ? input.value : 'CHAMP ABSENT', focused: document.activeElement === input, session: Number(new URLSearchParams(location.search).get('session')) }}); }})()" )).map_err(|error| error.to_string())?;
    let result = tokio::time::timeout(Duration::from_secs(2), receiver).await;
    OBSERVATIONS
        .get_or_init(Mutex::default)
        .lock()
        .unwrap()
        .remove(&token);
    result
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

async fn expect_capture(
    app: &AppHandle,
    value: &str,
    focused: bool,
) -> Result<CaptureState, String> {
    let start = Instant::now();
    loop {
        let state = capture_state(app).await?;
        if state.value == value && (!focused || state.focused) {
            return Ok(state);
        }
        if start.elapsed() > Duration::from_secs(2) {
            return Err("Brouillon ou focus de capture inattendu".into());
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

fn replace_probe_source(path: &std::path::Path, content: &str) -> Result<(), String> {
    // Présente au watcher la version complète : une écriture par troncature
    // expose d'abord un fichier vide valide et fausse le scénario « invalide ».
    let temporary = path.with_extension("probe.tmp");
    std::fs::write(&temporary, content).map_err(|error| error.to_string())?;
    std::fs::rename(temporary, path).map_err(|error| error.to_string())
}

async fn run(app: &AppHandle, directory: &std::path::Path) -> Result<Report, String> {
    let mode = WindowMode::requested();
    tokio::time::sleep(Duration::from_millis(200)).await;
    if !app.webview_windows().is_empty() {
        return Err("Une fenêtre est visible au démarrage".into());
    }
    let before = memory();
    let mut durations = vec![];
    let mut focus = vec![];
    let mut webview_sessions = vec![];
    for iteration in 0..3 {
        let (duration, focused) = open(app, Surface::Capture).await?;
        if app.webview_windows().len() != 1 {
            return Err("Ouverture dupliquée".into());
        }
        durations.push(duration);
        focus.push(focused);
        let state = expect_capture(app, "", true).await?;
        webview_sessions.push(state.session);
        app.get_webview_window("capture").ok_or("Capture absente")?.eval("(() => { const input = document.querySelector('#capture'); input.value = 'Brouillon de la sonde'; input.dispatchEvent(new Event('input', { bubbles: true })); })()").map_err(|error| error.to_string())?;
        expect_capture(app, "Brouillon de la sonde", true).await?;
        on_main(app, |app| {
            surfaces::open(app, Surface::Capture).map_err(|error| error.to_string())
        })
        .await?;
        expect_capture(app, "Brouillon de la sonde", true).await?;
        if mode == WindowMode::Hide && iteration == 2 {
            app.get_webview_window("capture").ok_or("Capture absente")?.eval("window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }))").map_err(|error| error.to_string())?;
            wait_until(|| {
                app.get_webview_window("capture")
                    .is_some_and(|window| !window.is_visible().unwrap_or(true))
            })
            .await?;
        } else {
            close(app, Surface::Capture, mode).await?;
        }
        if mode == WindowMode::Hide {
            expect_capture(app, "", false).await?;
        }
    }
    if mode == WindowMode::Hide
        && webview_sessions
            .iter()
            .any(|session| *session != webview_sessions[0])
    {
        return Err("La capture masquée a été recréée".into());
    }
    open(app, Surface::Panel).await?;
    open(app, Surface::Settings).await?;
    if visible_windows(app)? != 2
        || app.webview_windows().len() != if mode == WindowMode::Hide { 3 } else { 2 }
    {
        return Err("Deux surfaces distinctes attendues".into());
    }
    if mode == WindowMode::Hide {
        on_main(app, |app| {
            app.get_webview_window("settings")
                .ok_or("Paramètres absents")?
                .close()
                .map_err(|error| error.to_string())
        })
        .await?;
        wait_until(|| {
            app.get_webview_window("settings")
                .is_some_and(|window| !window.is_visible().unwrap_or(true))
        })
        .await?;
    } else {
        close(app, Surface::Settings, mode).await?;
    }
    close(app, Surface::Panel, mode).await?;
    if visible_windows(app)? != 0 {
        return Err("Une surface reste visible après fermeture".into());
    }
    if mode == WindowMode::Hide {
        on_main(app, |app| {
            surfaces::toggle_panel(app).map_err(|error| error.to_string())
        })
        .await?;
        wait_until(|| {
            app.get_webview_window("panel")
                .is_some_and(|window| window.is_visible().unwrap_or(false))
        })
        .await?;
        if app.webview_windows().len() != 3 {
            return Err("Le panneau a été dupliqué".into());
        }
        on_main(app, |app| {
            surfaces::toggle_panel(app).map_err(|error| error.to_string())
        })
        .await?;
        wait_until(|| {
            app.get_webview_window("panel")
                .is_some_and(|window| !window.is_visible().unwrap_or(true))
        })
        .await?;
    }
    tokio::time::sleep(Duration::from_secs(1)).await;
    let after = memory();
    tokio::time::sleep(Duration::from_secs(9)).await;
    let after_10_seconds = memory();

    let source = directory.join("tasks.md");
    if !std::fs::read_to_string(&source)
        .map_err(|error| error.to_string())?
        .is_empty()
    {
        return Err("La sonde requiert un fichier de tâches temporaire vide".into());
    }
    replace_probe_source(&source, "- [ ] Vérification du watcher\n")?;
    wait_until(|| desktop::desktop_status(app.clone()).task_count == 1).await?;
    replace_probe_source(&source, "Sauvegarde temporairement invalide")?;
    wait_until(|| {
        desktop::desktop_status(app.clone())
            .repository_error
            .is_some()
    })
    .await?;
    let preserved = desktop::desktop_status(app.clone()).task_count == 1;
    if !preserved {
        return Err("La version invalide a remplacé le dernier snapshot valide".into());
    }
    replace_probe_source(&source, "- [ ] Vérification du watcher\n")?;
    wait_until(|| {
        desktop::desktop_status(app.clone())
            .repository_error
            .is_none()
    })
    .await?;

    let secondary = tauri::async_runtime::spawn_blocking(|| {
        std::process::Command::new(std::env::current_exe().map_err(|error| error.to_string())?)
            .status()
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())??;
    if !secondary.success() {
        return Err("Le second lancement n'a pas quitté correctement".into());
    }
    Ok(Report {
        profile: "debug / surfaces prototypes / raccourcis et autostart non modifiés",
        window_mode: mode,
        session_type: std::env::var("XDG_SESSION_TYPE").ok(),
        startup_without_windows: true,
        capture_ready_ms: durations,
        native_focus_observed: focus,
        resident_memory_before_windows: before,
        resident_memory_after_windows: after,
        resident_memory_after_10_seconds: after_10_seconds,
        second_instance_exit_success: true,
        watcher_valid_reload: true,
        watcher_invalid_preserved: preserved,
        all_windows_destroyed: app.webview_windows().is_empty(),
        visible_windows_after_close: visible_windows(app)?,
        retained_native_windows_after_close: app.webview_windows().len(),
        capture_drafts_reset: true,
        capture_input_focused: true,
        repeated_open_keeps_draft: true,
        capture_webview_sessions: webview_sessions,
        panel_toggle_reopens_same_window: mode == WindowMode::Hide,
        native_close_masks_settings: mode == WindowMode::Hide,
        capture_escape_masks_window: mode == WindowMode::Hide,
    })
}

pub fn start(app: AppHandle, directory: PathBuf) {
    tauri::async_runtime::spawn(async move {
        let outcome = tokio::time::timeout(Duration::from_secs(40), run(&app, &directory)).await;
        let result = match outcome {
            Ok(Ok(report)) => {
                serde_json::to_string_pretty(&report).map_err(|error| error.to_string())
            }
            Ok(Err(error)) => Err(error),
            Err(_) => Err("La sonde a dépassé 40 secondes".into()),
        };
        match result {
            Ok(report) => {
                let path = directory.join("probe-report.json");
                match std::fs::write(&path, &report) {
                    Ok(()) => {
                        println!("Sonde desktop terminée : {}", path.display());
                        app.exit(0);
                    }
                    Err(error) => {
                        eprintln!("Sonde : {error}");
                        app.exit(1);
                    }
                }
            }
            Err(error) => {
                eprintln!("Sonde desktop : {error}");
                app.exit(1);
            }
        }
    });
}
