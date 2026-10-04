use crate::desktop::{self, Event};
use futures::{SinkExt, StreamExt, channel::mpsc::UnboundedReceiver};
use iced::{
    Element, Subscription, Task, event, keyboard,
    widget::{button, checkbox, column, container, scrollable, text, text_input},
    window,
};
use mystline::{
    config::{self, Config},
    repository::{Snapshot, TaskRepository},
};
use notify::RecommendedWatcher;
use std::{
    cell::RefCell,
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

static RECEIVER: OnceLock<Mutex<Option<UnboundedReceiver<Event>>>> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Surface {
    Capture,
    Panel,
    Settings,
}

struct State {
    config_dir: PathBuf,
    config: Config,
    repo: TaskRepository,
    snapshot: Snapshot,
    watcher: RecommendedWatcher,
    _tray: desktop::Tray,
    _hotkeys: Option<desktop::Shortcuts>,
    shortcut_status: (bool, bool),
    capture: Option<window::Id>,
    panel: Option<window::Id>,
    settings: Option<window::Id>,
    draft_path: String,
    error: Option<String>,
}

#[derive(Debug, Clone)]
enum Message {
    Desktop(Event),
    Opened(Surface, window::Id),
    Close(window::Id),
    PathChanged(String),
    SetSource(bool),
    Autostart(bool),
}

pub fn run(
    dir: PathBuf,
    config: Config,
    repo: TaskRepository,
) -> Result<(), Box<dyn std::error::Error>> {
    let watcher = desktop::watch(repo.path())?;
    let mut repo = repo;
    // Recheck after subscribing, closing the load/watch registration gap.
    if let Err(error) = repo.reload() {
        eprintln!("Mystline : édition extérieure ignorée : {error}");
    }
    let tray = desktop::Tray::new()?;
    let wayland = desktop::is_wayland();
    let hotkeys = if wayland {
        None
    } else {
        desktop::Shortcuts::new().ok()
    };
    let shortcut_status = hotkeys
        .as_ref()
        .map(|s| (s.capture_active, s.panel_active))
        .unwrap_or((false, false));
    let (sender, receiver) = futures::channel::mpsc::unbounded();
    desktop::install_events(sender);
    let _ = RECEIVER.set(Mutex::new(Some(receiver)));
    let state = State {
        config_dir: dir,
        draft_path: config.tasks_file.display().to_string(),
        config,
        snapshot: repo.snapshot(),
        repo,
        watcher,
        _tray: tray,
        _hotkeys: hotkeys,
        shortcut_status,
        capture: None,
        panel: None,
        settings: None,
        error: None,
    };
    let initial = RefCell::new(Some(state));
    iced::daemon(
        move || initial.borrow_mut().take().expect("initialisation unique"),
        update,
        view,
    )
    .title(|_: &State, _: window::Id| "Mystline".to_owned())
    .subscription(move |_| subscriptions(wayland))
    .run()?;
    Ok(())
}

fn desktop_stream() -> impl futures::Stream<Item = Message> {
    iced::stream::channel(100, async |mut output| {
        let rx = RECEIVER.get().and_then(|lock| lock.lock().ok()?.take());
        if let Some(mut rx) = rx {
            while let Some(event) = rx.next().await {
                if output.send(Message::Desktop(event)).await.is_err() {
                    break;
                }
            }
        }
    })
}

#[cfg(target_os = "linux")]
fn portal_stream() -> impl futures::Stream<Item = Message> {
    iced::stream::channel(1, async |mut output| {
        if let Err(error) = desktop::portal_shortcuts().await {
            let _ = output
                .send(Message::Desktop(Event::ShortcutStatus(false, false)))
                .await;
            eprintln!("Mystline : portail des raccourcis indisponible : {error}");
        }
        // Keep the subscription alive: failed registration must not be retried in a loop.
        futures::future::pending::<()>().await;
    })
}

fn subscriptions(wayland: bool) -> Subscription<Message> {
    #[allow(unused_mut)]
    let mut subs = vec![
        Subscription::run(desktop_stream),
        event::listen_with(|ev, _, id| match ev {
            iced::Event::Window(window::Event::CloseRequested) => Some(Message::Close(id)),
            iced::Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(keyboard::key::Named::Escape),
                ..
            }) => Some(Message::Close(id)),
            _ => None,
        }),
    ];
    #[cfg(target_os = "linux")]
    if wayland {
        subs.push(Subscription::run(portal_stream));
    }
    #[cfg(not(target_os = "linux"))]
    let _ = wayland;
    Subscription::batch(subs)
}

fn open(state: &mut State, surface: Surface) -> Task<Message> {
    let slot = match surface {
        Surface::Capture => &state.capture,
        Surface::Panel => &state.panel,
        Surface::Settings => &state.settings,
    };
    if let Some(id) = slot {
        return window::gain_focus(*id);
    }
    let settings = window::Settings {
        size: match surface {
            Surface::Capture => iced::Size::new(430.0, 115.0),
            Surface::Panel => iced::Size::new(430.0, 550.0),
            Surface::Settings => iced::Size::new(540.0, 320.0),
        },
        exit_on_close_request: false,
        decorations: surface == Surface::Settings,
        ..window::Settings::default()
    };
    let (_, task) = window::open(settings);
    task.map(move |id| Message::Opened(surface, id))
}

fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::Desktop(Event::Capture) => open(state, Surface::Capture),
        Message::Desktop(Event::Panel) => {
            if let Some(id) = state.panel.take() {
                window::close(id)
            } else {
                open(state, Surface::Panel)
            }
        }
        Message::Desktop(Event::Settings) => open(state, Surface::Settings),
        Message::Desktop(Event::Quit) => iced::exit(),
        Message::Desktop(Event::ShortcutStatus(a, b)) => {
            state.shortcut_status = (a, b);
            Task::none()
        }
        Message::Desktop(Event::FileChanged) => {
            desktop::acknowledge_file_change();
            match state.repo.reload() {
                Ok(changed) => {
                    if changed {
                        state.snapshot = state.repo.snapshot();
                    }
                    state.error = None;
                }
                Err(error) => {
                    eprintln!("Mystline : édition extérieure ignorée : {error}");
                    state.error = Some(error.to_string());
                }
            }
            Task::none()
        }
        Message::Opened(surface, id) => {
            *match surface {
                Surface::Capture => &mut state.capture,
                Surface::Panel => &mut state.panel,
                Surface::Settings => &mut state.settings,
            } = Some(id);
            window::gain_focus(id)
        }
        Message::Close(id) => {
            for slot in [&mut state.capture, &mut state.panel, &mut state.settings] {
                if *slot == Some(id) {
                    *slot = None;
                }
            }
            window::close(id)
        }
        Message::PathChanged(path) => {
            state.draft_path = path;
            Task::none()
        }
        Message::SetSource(create) => {
            let path = PathBuf::from(state.draft_path.trim());
            let result = (|| -> Result<(), Box<dyn std::error::Error>> {
                if !path.is_absolute() {
                    return Err("indiquer un chemin absolu".into());
                }
                if create && path.exists() {
                    return Err("le fichier existe déjà".into());
                }
                let mut candidate = TaskRepository::open(path, create)?;
                let watcher = desktop::watch(candidate.path())?;
                candidate.reload()?;
                let mut config = state.config.clone();
                config.tasks_file = candidate.path().canonicalize()?;
                config::save(&state.config_dir, &config)?;
                state.snapshot = candidate.snapshot();
                state.repo = candidate;
                state.watcher = watcher;
                state.config = config;
                state.draft_path = state.config.tasks_file.display().to_string();
                Ok(())
            })();
            state.error = result.err().map(|e| e.to_string());
            Task::none()
        }
        Message::Autostart(enabled) => {
            let mut updated = state.config.clone();
            updated.autostart = enabled;
            let result = desktop::set_autostart(&updated).and_then(|_| {
                config::save(&state.config_dir, &updated).map_err(std::io::Error::other)
            });
            match result {
                Ok(()) => {
                    state.config = updated;
                    state.error = None;
                }
                Err(error) => {
                    let _ = desktop::set_autostart(&state.config);
                    state.error = Some(error.to_string());
                }
            }
            Task::none()
        }
    }
}

fn view(state: &State, id: window::Id) -> Element<'_, Message> {
    let content: Element<'_, Message> = if state.capture == Some(id) {
        column![
            text("Capture rapide · saisie disponible avec F1"),
            button("Fermer").on_press(Message::Close(id))
        ]
        .spacing(12)
        .into()
    } else if state.panel == Some(id) {
        let mut list = column![text("Tâches")].spacing(10);
        for task in state.snapshot.tasks.iter().filter(|task| !task.completed) {
            list = list.push(text(format!("☐ {}", task.title)));
        }
        if !state.snapshot.tasks.iter().any(|task| !task.completed) {
            list = list.push(text("Aucune tâche à faire"));
        }
        scrollable(list).into()
    } else if state.settings == Some(id) {
        let (capture, panel) = state.shortcut_status;
        column![
            text("Paramètres"),
            checkbox(state.config.autostart)
                .label("Démarrer avec la session")
                .on_toggle(Message::Autostart),
            text("Fichier de tâches :"),
            text_input("/chemin/vers/tasks.md", &state.draft_path).on_input(Message::PathChanged),
            button("Utiliser le fichier existant").on_press(Message::SetSource(false)),
            button("Créer un nouveau fichier").on_press(Message::SetSource(true)),
            text(format!(
                "Raccourcis : capture {} ; panneau {}",
                if capture { "actif" } else { "inactif" },
                if panel { "actif" } else { "inactif" }
            )),
        ]
        .spacing(12)
        .into()
    } else {
        text("Mystline").into()
    };
    let mut frame = column![content].spacing(10);
    if let Some(error) = &state.error {
        frame = frame.push(text(error));
    }
    container(frame).padding(20).into()
}
