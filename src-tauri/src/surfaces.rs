use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Mutex, time::Instant};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Surface {
    Capture,
    Panel,
}

impl Surface {
    pub fn label(self) -> &'static str {
        match self {
            Self::Capture => "capture",
            Self::Panel => "panel",
        }
    }
    fn size(self, settings: bool) -> (f64, f64) {
        match self {
            Self::Capture => (460.0, 152.0),
            Self::Panel if settings => (520.0, 800.0),
            Self::Panel => (520.0, 680.0),
        }
    }
}

#[tauri::command]
pub fn capture_layout(window: tauri::WebviewWindow, height: f64) -> Result<(), String> {
    if window.label() != "capture" {
        return Err("Seule la capture peut se redimensionner.".into());
    }
    // Le frontend mesure son contenu ; conserver le bord supérieur et limiter à l’écran.
    let top = window
        .outer_position()
        .map_err(|error| error.to_string())?
        .y as f64;
    let available = window
        .current_monitor()
        .map_err(|error| error.to_string())?
        .map(|monitor| {
            (monitor.position().y as f64 + monitor.size().height as f64 - top)
                / monitor.scale_factor()
                - 32.0
        })
        .unwrap_or(600.0);
    window
        .set_size(tauri::LogicalSize::new(
            460.0,
            height.clamp(152.0, available.max(152.0)),
        ))
        .map_err(|error| error.to_string())
}

struct Session {
    id: u64,
    activation: u64,
    started: Instant,
    ready: bool,
    requested_visible: bool,
    settings: bool,
}

#[derive(Clone, Copy, Serialize)]
pub struct Activation {
    activation: u64,
    reset: bool,
    settings: bool,
}

enum Opening {
    Create(u64),
    Pending,
    Reuse(Activation),
}

#[derive(Default)]
pub struct Surfaces {
    sequence: u64,
    sessions: HashMap<Surface, Session>,
}

impl Surfaces {
    #[cfg(debug_assertions)]
    pub fn ready(&self, surface: Surface) -> bool {
        self.sessions
            .get(&surface)
            .is_some_and(|session| session.ready)
    }
    fn reserve(&mut self, surface: Surface, settings: bool) -> Option<u64> {
        if self.sessions.contains_key(&surface) {
            return None;
        }
        self.sequence += 1;
        let id = self.sequence;
        self.sessions.insert(
            surface,
            Session {
                id,
                activation: id,
                started: Instant::now(),
                ready: false,
                requested_visible: true,
                settings,
            },
        );
        Some(id)
    }
    fn request_open(&mut self, surface: Surface, settings: bool) -> Opening {
        if let Some(id) = self.reserve(surface, settings) {
            return Opening::Create(id);
        }
        let reset = !self.sessions[&surface].requested_visible
            || self.sessions[&surface].settings != settings;
        if reset {
            self.sequence += 1;
        }
        let session = self.sessions.get_mut(&surface).expect("surface réservée");
        session.requested_visible = true;
        session.settings = settings;
        if reset {
            session.activation = self.sequence;
        }
        if !session.ready {
            return Opening::Pending;
        }
        Opening::Reuse(Activation {
            activation: session.activation,
            reset,
            settings: session.settings,
        })
    }
    fn hide(&mut self, surface: Surface) -> Option<u64> {
        let session = self.sessions.get_mut(&surface)?;
        session.requested_visible = false;
        Some(session.activation)
    }
    fn hide_expected(&mut self, surface: Surface, id: u64, activation: u64) -> Option<u64> {
        if self
            .sessions
            .get(&surface)
            .is_none_or(|session| session.id != id || session.activation != activation)
        {
            return None;
        }
        self.hide(surface)
    }
    fn loaded(&mut self, surface: Surface, id: u64) -> Option<(Activation, std::time::Duration)> {
        let session = self.sessions.get_mut(&surface)?;
        if session.id != id || session.ready {
            return None;
        }
        session.ready = true;
        session.requested_visible.then_some((
            Activation {
                activation: session.activation,
                reset: true,
                settings: session.settings,
            },
            session.started.elapsed(),
        ))
    }
    fn release(&mut self, surface: Surface, id: u64) {
        if self
            .sessions
            .get(&surface)
            .is_some_and(|session| session.id == id)
        {
            self.sessions.remove(&surface);
        }
    }
}

pub fn open(app: &AppHandle, surface: Surface) -> tauri::Result<()> {
    open_with_view(app, surface, false)
}

/// Affiche les paramètres dans la fenêtre principale plutôt que dans une
/// WebView dédiée. Cela conserve une seule fenêtre pour le panneau et ses
/// réglages, tout en gardant la capture comme surface séparée.
pub fn show_settings(app: &AppHandle) -> tauri::Result<()> {
    open_with_view(app, Surface::Panel, true)
}

fn open_with_view(app: &AppHandle, surface: Surface, settings: bool) -> tauri::Result<()> {
    let request = app
        .state::<Mutex<Surfaces>>()
        .lock()
        .expect("surfaces")
        .request_open(surface, settings);
    let id = match request {
        Opening::Create(id) => id,
        Opening::Pending => return Ok(()),
        Opening::Reuse(activation) => {
            if let Some(window) = app.get_webview_window(surface.label()) {
                let (width, height) = surface.size(activation.settings);
                window.set_size(tauri::LogicalSize::new(width, height))?;
                window.emit("surface-opened", activation)?;
                window.show()?;
                window.set_focus()?;
            }
            return Ok(());
        }
    };
    let (width, height) = surface.size(settings);
    let height = app
        .primary_monitor()?
        .map(|monitor| {
            height.min((monitor.size().height as f64 / monitor.scale_factor() - 80.0).max(152.0))
        })
        .unwrap_or(height);
    let result = WebviewWindowBuilder::new(
        app,
        surface.label(),
        WebviewUrl::App(format!("index.html?surface={}&session={id}", surface.label()).into()),
    )
    .title("Mystline")
    .inner_size(width, height)
    .decorations(false)
    .resizable(false)
    .visible(false)
    .center()
    .build();
    match result {
        Ok(window) => {
            let handle = app.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    if let Err(error) = close(&handle, surface) {
                        tracing::error!(%error, "surface_hide_failed");
                    }
                }
                if matches!(event, tauri::WindowEvent::Destroyed) {
                    handle
                        .state::<Mutex<Surfaces>>()
                        .lock()
                        .expect("surfaces")
                        .release(surface, id);
                    tracing::info!(surface = surface.label(), session = id, "surface_destroyed");
                }
            });
            tracing::info!(surface = surface.label(), session = id, "surface_created");
            Ok(())
        }
        Err(error) => {
            app.state::<Mutex<Surfaces>>()
                .lock()
                .expect("surfaces")
                .release(surface, id);
            Err(error)
        }
    }
}

pub fn close(app: &AppHandle, surface: Surface) -> tauri::Result<()> {
    let activation = app
        .state::<Mutex<Surfaces>>()
        .lock()
        .expect("surfaces")
        .hide(surface);
    if let Some(window) = app.get_webview_window(surface.label()) {
        if let Some(activation) = activation {
            window.emit("surface-closed", activation)?;
        }
        window.hide()?;
        tracing::info!(surface = surface.label(), "surface_hidden");
    }
    Ok(())
}

pub fn toggle_panel(app: &AppHandle) -> tauri::Result<()> {
    if app
        .state::<Mutex<Surfaces>>()
        .lock()
        .expect("surfaces")
        .sessions
        .get(&Surface::Panel)
        .is_some_and(|session| session.requested_visible)
    {
        close(app, Surface::Panel)
    } else {
        open(app, Surface::Panel)
    }
}

fn finish_loading(window: tauri::WebviewWindow, session: u64) -> Result<(), String> {
    let app = window.app_handle().clone();
    let surface = match window.label() {
        "capture" => Surface::Capture,
        "panel" => Surface::Panel,
        _ => return Err("Surface inconnue".into()),
    };
    let loaded = {
        let state = app.state::<Mutex<Surfaces>>();
        let mut state = state.lock().map_err(|error| error.to_string())?;
        state.loaded(surface, session)
    };
    let Some((activation, duration)) = loaded else {
        return Ok(());
    };
    window
        .emit("surface-opened", activation)
        .map_err(|error| error.to_string())?;
    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())?;
    tracing::info!(
        surface = surface.label(),
        session,
        ready_ms = duration.as_millis(),
        "surface_ready"
    );
    Ok(())
}

#[tauri::command]
pub async fn surface_ready(window: tauri::WebviewWindow, session: u64) -> Result<(), String> {
    let app = window.app_handle().clone();
    let (sender, receiver) = tokio::sync::oneshot::channel();
    app.run_on_main_thread(move || {
        let _ = sender.send(finish_loading(window, session));
    })
    .map_err(|error| error.to_string())?;
    receiver.await.map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn open_surface(app: AppHandle, surface: Surface) -> Result<(), String> {
    let main = app.clone();
    let (sender, receiver) = tokio::sync::oneshot::channel();
    app.run_on_main_thread(move || {
        let _ = sender.send(open(&main, surface).map_err(|error| error.to_string()));
    })
    .map_err(|error| error.to_string())?;
    receiver.await.map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn open_settings(app: AppHandle) -> Result<(), String> {
    let main = app.clone();
    let (sender, receiver) = tokio::sync::oneshot::channel();
    app.run_on_main_thread(move || {
        let _ = sender.send(show_settings(&main).map_err(|error| error.to_string()));
    })
    .map_err(|error| error.to_string())?;
    receiver.await.map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn close_surface(
    window: tauri::WebviewWindow,
    session: u64,
    activation: u64,
) -> Result<(), String> {
    let surface = match window.label() {
        "capture" => Surface::Capture,
        "panel" => Surface::Panel,
        _ => return Err("Surface inconnue".into()),
    };
    let app = window.app_handle().clone();
    let main = app.clone();
    let (sender, receiver) = tokio::sync::oneshot::channel();
    app.run_on_main_thread(move || {
        let result = (|| {
            let hidden = main
                .state::<Mutex<Surfaces>>()
                .lock()
                .map_err(|error| error.to_string())?
                .hide_expected(surface, session, activation);
            if let Some(activation) = hidden
                && let Some(window) = main.get_webview_window(surface.label())
            {
                window
                    .emit("surface-closed", activation)
                    .map_err(|error| error.to_string())?;
                window.hide().map_err(|error| error.to_string())?;
            }
            Ok(())
        })();
        let _ = sender.send(result);
    })
    .map_err(|error| error.to_string())?;
    receiver.await.map_err(|error| error.to_string())?
}

/// Ferme la surface dont provient une action explicite de son interface.
///
/// Contrairement à `close_surface`, cette commande ne reçoit pas une activation
/// potentiellement obsolète : le clic sur la croix doit toujours masquer la
/// fenêtre que l'utilisateur voit. Les retours asynchrones conservent la
/// commande identifiée ci-dessus pour ne pas fermer une surface rouverte.
#[tauri::command]
pub async fn close_current_surface(window: tauri::WebviewWindow) -> Result<(), String> {
    let surface = match window.label() {
        "capture" => Surface::Capture,
        "panel" => Surface::Panel,
        _ => return Err("Surface inconnue".into()),
    };
    let app = window.app_handle().clone();
    let main = app.clone();
    let (sender, receiver) = tokio::sync::oneshot::channel();
    app.run_on_main_thread(move || {
        let _ = sender.send(close(&main, surface).map_err(|error| error.to_string()));
    })
    .map_err(|error| error.to_string())?;
    receiver.await.map_err(|error| error.to_string())?
}

#[cfg(test)]
#[path = "surfaces_tests.rs"]
mod migration_tests;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn un_clic_repete_reserve_une_seule_fenetre() {
        // RG-F0-07 — Quand le tray est activé plusieurs fois avant le chargement Vue.
        let mut surfaces = Surfaces::default();
        let first = surfaces.reserve(Surface::Panel, false).unwrap();
        assert!(surfaces.reserve(Surface::Panel, false).is_none());
        surfaces.release(Surface::Panel, first);
        assert!(surfaces.reserve(Surface::Panel, false).is_some());
    }
    #[test]
    fn la_fermeture_retardee_ne_supprime_pas_la_nouvelle_session() {
        let mut surfaces = Surfaces::default();
        let old = surfaces.reserve(Surface::Capture, false).unwrap();
        surfaces.release(Surface::Capture, old);
        let new = surfaces.reserve(Surface::Capture, false).unwrap();
        surfaces.release(Surface::Capture, old);
        assert_eq!(surfaces.sessions[&Surface::Capture].id, new);
    }
    #[test]
    fn fermer_avant_le_chargement_ne_reaffiche_pas_la_fenetre() {
        let mut surfaces = Surfaces::default();
        let native = surfaces.reserve(Surface::Capture, false).unwrap();
        surfaces.hide(Surface::Capture);
        assert!(surfaces.loaded(Surface::Capture, native).is_none());
        let Opening::Reuse(activation) = surfaces.request_open(Surface::Capture, false) else {
            panic!("fenêtre à réutiliser");
        };
        assert!(activation.reset);
        assert_eq!(surfaces.sessions[&Surface::Capture].id, native);
    }
    #[test]
    fn redonner_le_focus_conserve_le_brouillon_et_reouvrir_change_lactivation() {
        let mut surfaces = Surfaces::default();
        let native = surfaces.reserve(Surface::Capture, false).unwrap();
        let (first, _) = surfaces.loaded(Surface::Capture, native).unwrap();
        let Opening::Reuse(focus) = surfaces.request_open(Surface::Capture, false) else {
            panic!("surface prête");
        };
        assert!(!focus.reset);
        assert_eq!(first.activation, focus.activation);
        surfaces.hide(Surface::Capture);
        let Opening::Reuse(reopened) = surfaces.request_open(Surface::Capture, false) else {
            panic!("surface à réouvrir");
        };
        assert!(reopened.reset);
        assert_ne!(first.activation, reopened.activation);
        assert_eq!(surfaces.sessions.len(), 1);
    }
    #[test]
    fn les_parametres_remplacent_le_panneau_sans_creer_de_session() {
        let mut surfaces = Surfaces::default();
        let native = surfaces.reserve(Surface::Panel, false).unwrap();
        let (panel, _) = surfaces.loaded(Surface::Panel, native).unwrap();

        let Opening::Reuse(settings) = surfaces.request_open(Surface::Panel, true) else {
            panic!("panneau à réutiliser");
        };

        assert!(settings.reset);
        assert!(settings.settings);
        assert_ne!(panel.activation, settings.activation);
        assert_eq!(surfaces.sessions.len(), 1);
        assert_eq!(surfaces.sessions[&Surface::Panel].id, native);
    }
}
