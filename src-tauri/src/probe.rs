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
    functional_capture_confirmed: bool,
    invalid_capture_preserves_draft: bool,
    completion_and_history_persisted: bool,
    stale_ipc_rejected: bool,
    source_switch_transactional: bool,
    isolated_settings_persisted: bool,
    settings_layout: serde_json::Value,
}

struct CaptureState {
    value: String,
    focused: bool,
    session: u64,
}
type ObservationSenders = Mutex<HashMap<u64, tokio::sync::oneshot::Sender<CaptureState>>>;
static OBSERVATIONS: OnceLock<ObservationSenders> = OnceLock::new();
static OBSERVATION_SEQUENCE: AtomicU64 = AtomicU64::new(0);

type DomObservationSenders = Mutex<HashMap<u64, tokio::sync::oneshot::Sender<serde_json::Value>>>;
static DOM_OBSERVATIONS: OnceLock<DomObservationSenders> = OnceLock::new();

#[tauri::command]
pub fn probe_dom_state(
    window: tauri::WebviewWindow,
    token: u64,
    value: serde_json::Value,
) -> Result<(), String> {
    if directory().is_none() || !matches!(window.label(), "capture" | "panel") {
        return Err("Observation réservée à la sonde".into());
    }
    DOM_OBSERVATIONS
        .get_or_init(Mutex::default)
        .lock()
        .map_err(|error| error.to_string())?
        .remove(&token)
        .ok_or("Observation expirée")?
        .send(value)
        .map_err(|_| "Observation annulée".into())
}

/// Observe la vraie WebView ou attend une commande IPC, sans API de production ajoutée.
async fn dom_state(
    app: &AppHandle,
    surface: Surface,
    expression: &str,
) -> Result<serde_json::Value, String> {
    let token = OBSERVATION_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let (sender, receiver) = tokio::sync::oneshot::channel();
    DOM_OBSERVATIONS
        .get_or_init(Mutex::default)
        .lock()
        .unwrap()
        .insert(token, sender);
    let expression = format!(
        "(async () => {{ let value; try {{ value = await ({expression}); if (value === undefined) value = null; }} catch (error) {{ value = {{ probeError: String(error) }}; }} window.__TAURI_INTERNALS__.invoke('probe_dom_state', {{ token: {token}, value }}); }})()"
    );
    app.get_webview_window(surface.label())
        .ok_or("Surface absente")?
        .eval(expression)
        .map_err(|error| error.to_string())?;
    let result = tokio::time::timeout(Duration::from_secs(3), receiver).await;
    DOM_OBSERVATIONS
        .get_or_init(Mutex::default)
        .lock()
        .unwrap()
        .remove(&token);
    let value = result
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())?;
    if let Some(error) = value.get("probeError") {
        return Err(error.to_string());
    }
    Ok(value)
}

async fn expect_dom(
    app: &AppHandle,
    surface: Surface,
    expression: &str,
    expected: serde_json::Value,
) -> Result<(), String> {
    let start = Instant::now();
    loop {
        let value = dom_state(app, surface, expression).await?;
        if value == expected {
            return Ok(());
        }
        if start.elapsed() > Duration::from_secs(3) {
            return Err(format!(
                "DOM inattendu : attendu {expected}, obtenu {value}"
            ));
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

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

async fn functional_ipc(app: &AppHandle, directory: &std::path::Path) -> Result<(), String> {
    use serde_json::json;
    let source_path = directory.join("tasks.md");
    // La vue est demandée depuis la vraie WebView, comme par le client Vue.
    let view_expression = "window.__TAURI_INTERNALS__.invoke('task_view', { request: { query: '', tag: null, history: false } })";
    let before = dom_state(app, Surface::Panel, view_expression).await?;
    if before["rows"].as_array().is_none_or(|rows| rows.len() != 2) {
        return Err("La capture confirmée n'apparaît pas dans la projection IPC".into());
    }
    let previous_source = before["source"].clone();
    let toggle = format!(
        "window.__TAURI_INTERNALS__.invoke('task_toggle', {{ source: {}, position: 1, request: {{ query: '', tag: null, history: false }} }})",
        previous_source
    );
    dom_state(app, Surface::Panel, &toggle).await?;
    let completed = dom_state(app, Surface::Panel, view_expression).await?;
    if completed["busy"] != false
        || completed["version"].as_u64() <= before["version"].as_u64()
        || completed["rows"][1]["completed"] != true
    {
        return Err("Complétion IPC non confirmée ou version non monotone".into());
    }
    let persisted = std::fs::read_to_string(&source_path).map_err(|error| error.to_string())?;
    if !persisted.contains("- [x] Capture fonctionnelle #mission @completed(") {
        return Err("La réponse de complétion précède son écriture persistée".into());
    }
    let history = dom_state(app, Surface::Panel, "window.__TAURI_INTERNALS__.invoke('task_view', { request: { query: 'fonctionnelle', tag: 'MISSION', history: true } })").await?;
    if history["rows"]
        .as_array()
        .is_none_or(|rows| rows.len() != 1)
        || history["rows"][0]["title"] != "Capture fonctionnelle"
    {
        return Err("L'historique IPC n'applique pas la recherche et le tag".into());
    }
    let stale_toggle = format!(
        "window.__TAURI_INTERNALS__.invoke('task_toggle', {{ source: {}, position: 0, request: {{ query: '', tag: null, history: false }} }}).then(() => false, () => true)",
        previous_source
    );
    if dom_state(app, Surface::Panel, &stale_toggle).await? != true {
        return Err("Une ancienne révision IPC modifie le fichier".into());
    }
    // Le réglage invalide n'active pas sa source et n'écrit pas son chemin.
    let invalid_path = directory.join("invalid.md");
    std::fs::write(&invalid_path, "Sauvegarde invalide").map_err(|error| error.to_string())?;
    let invalid_switch = format!(
        "window.__TAURI_INTERNALS__.invoke('settings_source', {{ path: {}, create: false }}).then(() => false, () => true)",
        json!(invalid_path.to_string_lossy())
    );
    if dom_state(app, Surface::Panel, &invalid_switch).await? != true
        || std::fs::read_to_string(&source_path).map_err(|error| error.to_string())? != persisted
    {
        return Err("Le changement de source invalide n'est pas transactionnel".into());
    }
    let next_path = directory.join("second.md");
    std::fs::write(&next_path, "- [ ] Nouvelle source\n").map_err(|error| error.to_string())?;
    let switch = format!(
        "window.__TAURI_INTERNALS__.invoke('settings_source', {{ path: {}, create: false }})",
        json!(next_path.to_string_lossy())
    );
    dom_state(app, Surface::Panel, &switch).await?;
    let switched = dom_state(app, Surface::Panel, view_expression).await?;
    if switched["source"]["generation"] == completed["source"]["generation"]
        || switched["rows"][0]["title"] != "Nouvelle source"
    {
        return Err("La source n'a pas changé de génération".into());
    }
    let stale_source = format!(
        "window.__TAURI_INTERNALS__.invoke('task_toggle', {{ source: {}, position: 0, request: {{ query: '', tag: null, history: false }} }}).then(() => false, () => true)",
        completed["source"]
    );
    if dom_state(app, Surface::Panel, &stale_source).await? != true
        || std::fs::read_to_string(&next_path).map_err(|error| error.to_string())?
            != "- [ ] Nouvelle source\n"
    {
        return Err("Une commande de l'ancienne source touche le nouveau fichier".into());
    }
    let config = mystline::config::load_or_create(directory).map_err(|error| error.to_string())?;
    if config.tasks_file != next_path
        || std::fs::read_to_string(&source_path).map_err(|error| error.to_string())? != persisted
    {
        return Err("Le choix de source n'est pas persisté ou altère l'ancien fichier".into());
    }
    // Les réglages du probe restent dans son dossier et les intégrations OS sont inhibées.
    let collision = format!(
        "window.__TAURI_INTERNALS__.invoke('settings_shortcut', {{ capture: true, value: {} }}).then(() => false, () => true)",
        json!(config.shortcuts.panel)
    );
    if dom_state(app, Surface::Panel, &collision).await? != true {
        return Err("Une collision interne de raccourci n'est pas refusée".into());
    }
    let after_collision =
        mystline::config::load_or_create(directory).map_err(|error| error.to_string())?;
    if after_collision.shortcuts != config.shortcuts {
        return Err("Une collision de raccourci change la configuration persistée".into());
    }
    dom_state(app, Surface::Panel, "window.__TAURI_INTERNALS__.invoke('settings_shortcut', { capture: true, value: 'Alt+Shift+C' })").await?;
    dom_state(
        app,
        Surface::Panel,
        "window.__TAURI_INTERNALS__.invoke('settings_autostart', { enabled: false })",
    )
    .await?;
    let updated_config =
        mystline::config::load_or_create(directory).map_err(|error| error.to_string())?;
    if updated_config.shortcuts.capture != "Alt+Shift+C"
        || updated_config.shortcuts.panel != config.shortcuts.panel
        || updated_config.autostart
    {
        return Err("Les réglages isolés ne sont pas persistés indépendamment".into());
    }
    #[cfg(target_os = "linux")]
    {
        // Une source saisie relative est normalisée avant configuration et watcher.
        let relative_path = directory.join("relative.md");
        std::fs::write(&relative_path, "- [ ] Source relative\n")
            .map_err(|error| error.to_string())?;
        let cwd = std::env::current_dir().map_err(|error| error.to_string())?;
        let parents = cwd
            .components()
            .filter(|component| matches!(component, std::path::Component::Normal(_)))
            .count();
        let relative = format!(
            "{}{}",
            "../".repeat(parents),
            relative_path
                .strip_prefix("/")
                .map_err(|error| error.to_string())?
                .display()
        );
        let switch = format!(
            "window.__TAURI_INTERNALS__.invoke('settings_source', {{ path: {}, create: false }})",
            json!(relative)
        );
        dom_state(app, Surface::Panel, &switch).await?;
        let configured =
            mystline::config::load_or_create(directory).map_err(|error| error.to_string())?;
        if !configured.tasks_file.is_absolute() {
            return Err("Une source relative reste dépendante du répertoire de lancement".into());
        }
        replace_probe_source(
            &relative_path,
            "- [ ] Source relative\n- [ ] Modification externe\n",
        )?;
        wait_until(|| desktop::desktop_status(app.clone()).task_count == 2).await?;
        let watched = dom_state(app, Surface::Panel, view_expression).await?;
        if watched["rows"][1]["title"] != "Modification externe" {
            return Err("Le watcher ne suit pas la source initialement relative".into());
        }
        let actual_path = directory.join("symlink-target.md");
        let linked_path = directory.join("symlink-source.md");
        std::fs::write(&actual_path, "- [ ] Cible originale\n")
            .map_err(|error| error.to_string())?;
        std::os::unix::fs::symlink(&actual_path, &linked_path)
            .map_err(|error| error.to_string())?;
        let switch = format!(
            "window.__TAURI_INTERNALS__.invoke('settings_source', {{ path: {}, create: false }})",
            json!(linked_path.to_string_lossy())
        );
        dom_state(app, Surface::Panel, &switch).await?;
        dom_state(app, Surface::Panel, "window.__TAURI_INTERNALS__.invoke('task_create', { draft: 'Ajout via lien', session: Number(new URLSearchParams(location.search).get('session')), activation: 999999 })").await?;
        let configured =
            mystline::config::load_or_create(directory).map_err(|error| error.to_string())?;
        let reopened = mystline::repository::TaskRepository::open(actual_path.clone(), false)
            .map_err(|error| error.to_string())?
            .snapshot();
        if configured.tasks_file != actual_path
            || !linked_path
                .symlink_metadata()
                .map_err(|error| error.to_string())?
                .file_type()
                .is_symlink()
            || reopened.tasks.len() != 2
            || reopened.tasks[1].title != "Ajout via lien"
        {
            return Err(
                "Une écriture via source symlink diverge de la configuration ou remplace le lien"
                    .into(),
            );
        }
    }
    Ok(())
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
    on_main(app, |app| {
        surfaces::show_settings(app).map_err(|error| error.to_string())
    })
    .await?;
    let settings_layout = dom_state(app, Surface::Panel, "(() => { const surface = document.querySelector('.surface'); const buttons = [...document.querySelectorAll('button')].filter(button => button.textContent.trim() === 'Enregistrer'); const last = buttons[buttons.length - 1]; return { viewportHeight: innerHeight, surfaceClientHeight: surface.clientHeight, surfaceScrollHeight: surface.scrollHeight, lastSaveButtonBottom: last ? last.getBoundingClientRect().bottom : null }; })()").await?;
    if visible_windows(app)? != 1
        || app.webview_windows().len() != if mode == WindowMode::Hide { 2 } else { 1 }
    {
        return Err("Les paramètres doivent réutiliser la fenêtre du panneau".into());
    }
    if mode == WindowMode::Hide {
        on_main(app, |app| {
            app.get_webview_window("panel")
                .ok_or("Paramètres absents")?
                .close()
                .map_err(|error| error.to_string())
        })
        .await?;
        wait_until(|| {
            app.get_webview_window("panel")
                .is_some_and(|window| !window.is_visible().unwrap_or(true))
        })
        .await?;
    } else {
        close(app, Surface::Panel, mode).await?;
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
        if app.webview_windows().len() != 2 {
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

    // Soumet le vrai formulaire de capture et attend le résultat visible/persisté.
    open(app, Surface::Capture).await?;
    expect_capture(app, "", true).await?;
    app.get_webview_window("capture").ok_or("Capture absente")?.eval("(() => { const input = document.querySelector('#capture'); input.value = 'Capture invalide /d impossible'; input.dispatchEvent(new Event('input', { bubbles: true })); input.closest('form').requestSubmit(); })()").map_err(|error| error.to_string())?;
    expect_dom(
        app,
        Surface::Capture,
        "Boolean(document.querySelector('[role=alert]'))",
        serde_json::json!(true),
    )
    .await?;
    expect_capture(app, "Capture invalide /d impossible", true).await?;
    if std::fs::read_to_string(&source).map_err(|error| error.to_string())?
        != "- [ ] Vérification du watcher\n"
    {
        return Err("La capture invalide écrit dans le fichier".into());
    }
    app.get_webview_window("capture").ok_or("Capture absente")?.eval("(() => { const input = document.querySelector('#capture'); input.value = 'Capture fonctionnelle #mission'; input.dispatchEvent(new Event('input', { bubbles: true })); input.closest('form').requestSubmit(); })()").map_err(|error| error.to_string())?;
    wait_until(|| {
        app.get_webview_window("capture")
            .is_some_and(|window| !window.is_visible().unwrap_or(true))
    })
    .await?;
    if !std::fs::read_to_string(&source)
        .map_err(|error| error.to_string())?
        .contains("- [ ] Capture fonctionnelle #mission\n")
    {
        return Err("La capture s'est fermée avant son écriture".into());
    }
    open(app, Surface::Panel).await?;
    expect_dom(
        app,
        Surface::Panel,
        "document.body.textContent.includes('Capture fonctionnelle')",
        serde_json::json!(true),
    )
    .await?;
    functional_ipc(app, directory).await?;
    close(app, Surface::Capture, mode).await?;
    close(app, Surface::Panel, mode).await?;

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
        profile: "debug / portage fonctionnel / raccourcis et autostart non modifiés",
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
        functional_capture_confirmed: true,
        invalid_capture_preserves_draft: true,
        completion_and_history_persisted: true,
        stale_ipc_rejected: true,
        source_switch_transactional: true,
        isolated_settings_persisted: true,
        settings_layout,
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
