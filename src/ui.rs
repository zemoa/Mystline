use crate::desktop::{self, Event};
use futures::{SinkExt, StreamExt, channel::mpsc::UnboundedReceiver};
use iced::{
    Element, Rectangle, Subscription, Task, Vector,
    advanced::widget::{
        self,
        operation::{self, Outcome},
    },
    event, keyboard,
    widget::{button, checkbox, column, container, scrollable, text, text_input},
    window,
};
use mystline::{
    application,
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

#[derive(Default)]
struct QuickCapture {
    draft: String,
    error: Option<String>,
}

impl QuickCapture {
    fn open(&mut self) {
        self.draft.clear();
        self.error = None;
    }

    fn edit(&mut self, draft: String) {
        self.draft = draft;
        self.error = None;
    }

    fn submit(&mut self) -> Option<String> {
        let title = self.draft.trim();
        if title.is_empty() {
            self.error = Some("Saisir un titre".into());
            None
        } else {
            Some(title.to_owned())
        }
    }

    fn fail(&mut self, error: String) {
        self.error = Some(error);
    }
}

#[derive(Default)]
struct PanelNavigation {
    // Ordinal among active tasks, not the repository index.
    selected: Option<usize>,
    focused: bool,
}

impl PanelNavigation {
    fn open(&mut self, count: usize) {
        self.focused = true;
        self.selected = (count > 0).then_some(0);
    }

    fn refresh(&mut self, count: usize) {
        self.selected = if count == 0 {
            None
        } else {
            Some(self.selected.unwrap_or(0).min(count - 1))
        };
    }

    fn next(&mut self, count: usize) {
        self.refresh(count);
        if let Some(selected) = &mut self.selected {
            *selected = (*selected + 1).min(count - 1);
        }
    }

    fn previous(&mut self) {
        if let Some(selected) = &mut self.selected {
            *selected = selected.saturating_sub(1);
        }
    }

    fn selected_index(&self, active: &[usize]) -> Option<usize> {
        self.selected
            .and_then(|selected| active.get(selected).copied())
    }
}

fn active_count(snapshot: &Snapshot) -> usize {
    snapshot.tasks.iter().filter(|task| !task.completed).count()
}

fn panel_active_indices(snapshot: &Snapshot) -> Vec<usize> {
    snapshot
        .tasks
        .iter()
        .enumerate()
        .filter_map(|(index, task)| (!task.completed).then_some(index))
        .collect()
}

fn replace_panel_snapshot(
    snapshot: &mut Snapshot,
    navigation: &mut PanelNavigation,
    confirmed: Snapshot,
) {
    *snapshot = confirmed;
    navigation.refresh(active_count(snapshot));
}

fn scroll_offset_for_row(top: f32, height: f32, offset: f32, viewport_height: f32) -> Option<f32> {
    if top < offset || height > viewport_height {
        Some(top)
    } else if top + height > offset + viewport_height {
        Some(top + height - viewport_height)
    } else {
        None
    }
}

// Inspect actual row bounds, so long wrapped titles and resized panels scroll correctly.
struct ScrollSelected {
    viewport: Option<(Rectangle, Rectangle, Vector)>,
    row: Option<Rectangle>,
}

impl widget::Operation<Message> for ScrollSelected {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn widget::Operation<Message>)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        id: Option<&widget::Id>,
        bounds: Rectangle,
        content_bounds: Rectangle,
        translation: Vector,
        _state: &mut dyn operation::Scrollable,
    ) {
        if id == Some(&widget::Id::new("panel-list")) {
            self.viewport = Some((bounds, content_bounds, translation));
        }
    }

    fn container(&mut self, id: Option<&widget::Id>, bounds: Rectangle) {
        if id == Some(&widget::Id::new("panel-selected")) {
            self.row = Some(bounds);
        }
    }

    fn finish(&self) -> Outcome<Message> {
        let Some((viewport, content, translation)) = self.viewport else {
            return Outcome::None;
        };
        let Some(row) = self.row else {
            return Outcome::None;
        };
        let top = row.y - content.y;
        let Some(y) = scroll_offset_for_row(top, row.height, translation.y, viewport.height) else {
            return Outcome::None;
        };
        Outcome::Chain(Box::new(operation::scrollable::scroll_to(
            widget::Id::new("panel-list"),
            operation::scrollable::AbsoluteOffset {
                x: None,
                y: Some(y),
            },
        )))
    }
}

fn reveal_selected() -> Task<Message> {
    widget::operate(ScrollSelected {
        viewport: None,
        row: None,
    })
}

fn save_capture(capture: &mut QuickCapture, repo: &mut TaskRepository) -> Option<Snapshot> {
    let title = capture.submit()?;
    match application::create_task(repo, &title) {
        Ok(snapshot) => Some(snapshot),
        Err(error) => {
            capture.fail(error.to_string());
            None
        }
    }
}

// Returns whether this message confirmed a task and should close the capture window.
fn update_capture(
    message: Message,
    current: Option<window::Id>,
    capture: &mut QuickCapture,
    repo: &mut TaskRepository,
    snapshot: &mut Snapshot,
    navigation: &mut PanelNavigation,
) -> bool {
    match message {
        Message::CaptureChanged(id, draft) if current == Some(id) => capture.edit(draft),
        Message::CaptureSubmitted(id) if current == Some(id) => {
            if let Some(confirmed) = save_capture(capture, repo) {
                replace_panel_snapshot(snapshot, navigation, confirmed);
                return true;
            }
            // A conflicting external edit may have refreshed the repository cache.
            replace_panel_snapshot(snapshot, navigation, repo.snapshot());
        }
        _ => (),
    }
    false
}

enum WindowTransition {
    Reserve(window::Id),
    Opened(window::Id),
    Close(window::Id),
    Closed(window::Id),
}

// Keep the ID reserved even before Iced's asynchronous Opened reply arrives.
fn window_transition(slot: &mut Option<window::Id>, action: WindowTransition) -> bool {
    match action {
        WindowTransition::Reserve(id) if slot.is_none() => {
            *slot = Some(id);
            true
        }
        WindowTransition::Opened(id) => *slot == Some(id),
        WindowTransition::Close(id) | WindowTransition::Closed(id) if *slot == Some(id) => {
            *slot = None;
            true
        }
        _ => false,
    }
}

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
    quick_capture: QuickCapture,
    navigation: PanelNavigation,
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
    Closed(window::Id),
    CaptureChanged(window::Id, String),
    CaptureSubmitted(window::Id),
    PanelMove(window::Id, bool),
    PanelFocus(window::Id, bool),
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
        quick_capture: QuickCapture::default(),
        navigation: PanelNavigation::default(),
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
        event::listen_with(|ev, _status, id| match ev {
            iced::Event::Window(window::Event::CloseRequested) => Some(Message::Close(id)),
            iced::Event::Window(window::Event::Closed) => Some(Message::Closed(id)),
            iced::Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(keyboard::key::Named::Escape),
                ..
            }) => Some(Message::Close(id)),
            iced::Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(keyboard::key::Named::ArrowDown),
                modifiers,
                ..
            }) if modifiers.is_empty() => Some(Message::PanelMove(id, true)),
            iced::Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(keyboard::key::Named::ArrowUp),
                modifiers,
                ..
            }) if modifiers.is_empty() => Some(Message::PanelMove(id, false)),
            iced::Event::Window(window::Event::Focused) => Some(Message::PanelFocus(id, true)),
            iced::Event::Window(window::Event::Unfocused) => Some(Message::PanelFocus(id, false)),
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
    let current = match surface {
        Surface::Capture => state.capture,
        Surface::Panel => state.panel,
        Surface::Settings => state.settings,
    };
    if let Some(id) = current {
        return if surface == Surface::Capture {
            capture_focus(id)
        } else {
            window::gain_focus(id)
        };
    }
    if surface == Surface::Capture {
        state.quick_capture.open();
    } else if surface == Surface::Panel {
        state.navigation.open(active_count(&state.snapshot));
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
    let (id, task) = window::open(settings);
    window_transition(
        match surface {
            Surface::Capture => &mut state.capture,
            Surface::Panel => &mut state.panel,
            Surface::Settings => &mut state.settings,
        },
        WindowTransition::Reserve(id),
    );
    task.map(move |id| Message::Opened(surface, id))
}

fn capture_focus(id: window::Id) -> Task<Message> {
    window::gain_focus(id).chain(iced::widget::operation::focus("capture-title"))
}

fn close(state: &mut State, id: window::Id) -> Task<Message> {
    for slot in [&mut state.capture, &mut state.panel, &mut state.settings] {
        if window_transition(slot, WindowTransition::Close(id)) {
            return window::close(id);
        }
    }
    Task::none()
}

fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::Desktop(Event::Capture) => open(state, Surface::Capture),
        Message::Desktop(Event::OpenPanel) => open(state, Surface::Panel),
        Message::Desktop(Event::Panel) => {
            if let Some(id) = state.panel {
                close(state, id)
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
                        replace_panel_snapshot(
                            &mut state.snapshot,
                            &mut state.navigation,
                            state.repo.snapshot(),
                        );
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
            if !window_transition(
                match surface {
                    Surface::Capture => &mut state.capture,
                    Surface::Panel => &mut state.panel,
                    Surface::Settings => &mut state.settings,
                },
                WindowTransition::Opened(id),
            ) {
                return window::close(id);
            }
            if surface == Surface::Capture {
                capture_focus(id)
            } else {
                window::gain_focus(id)
            }
        }
        Message::Close(id) => close(state, id),
        Message::Closed(id) => {
            for slot in [&mut state.capture, &mut state.panel, &mut state.settings] {
                window_transition(slot, WindowTransition::Closed(id));
            }
            Task::none()
        }
        Message::CaptureChanged(id, draft) => {
            let had_error = state.quick_capture.error.is_some();
            update_capture(
                Message::CaptureChanged(id, draft),
                state.capture,
                &mut state.quick_capture,
                &mut state.repo,
                &mut state.snapshot,
                &mut state.navigation,
            );
            if had_error && state.capture == Some(id) {
                window::resize(id, iced::Size::new(430.0, 115.0))
            } else {
                Task::none()
            }
        }
        Message::CaptureSubmitted(id) => {
            if update_capture(
                Message::CaptureSubmitted(id),
                state.capture,
                &mut state.quick_capture,
                &mut state.repo,
                &mut state.snapshot,
                &mut state.navigation,
            ) {
                close(state, id)
            } else if state.capture == Some(id) && state.quick_capture.error.is_some() {
                window::resize(id, iced::Size::new(430.0, 205.0))
            } else {
                Task::none()
            }
        }
        Message::PanelMove(id, down) => {
            if state.panel == Some(id) && state.navigation.focused {
                if down {
                    state.navigation.next(active_count(&state.snapshot));
                } else {
                    state.navigation.previous();
                }
                return reveal_selected();
            }
            Task::none()
        }
        Message::PanelFocus(id, focused) => {
            if state.panel == Some(id) {
                state.navigation.focused = focused;
            }
            Task::none()
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
                replace_panel_snapshot(
                    &mut state.snapshot,
                    &mut state.navigation,
                    candidate.snapshot(),
                );
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
        let mut capture = column![
            text("Capture rapide"),
            text_input("Titre de la tâche", &state.quick_capture.draft)
                .id("capture-title")
                .on_input(move |draft| Message::CaptureChanged(id, draft))
                .on_submit(Message::CaptureSubmitted(id)),
        ]
        .spacing(12);
        if let Some(error) = &state.quick_capture.error {
            capture = capture.push(scrollable(text(error)).height(75));
        }
        capture.into()
    } else if state.panel == Some(id) {
        let mut list = column![text("Tâches")].spacing(10);
        let active = panel_active_indices(&state.snapshot);
        let selected = state.navigation.selected_index(&active);
        for index in &active {
            let task = &state.snapshot.tasks[*index];
            let marker = if selected == Some(*index) { "▸" } else { " " };
            let row = container(text(format!("{marker} ☐ {}", task.title)));
            list = list.push(if selected == Some(*index) {
                row.id("panel-selected")
            } else {
                row
            });
        }
        if active.is_empty() {
            list = list.push(text("Aucune tâche à faire"));
        }
        scrollable(list).id("panel-list").into()
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
    if let Some(error) = &state.error
        && state.settings == Some(id)
    {
        frame = frame.push(text(error));
    }
    container(frame).padding(20).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_reopens_empty_and_validates_without_losing_original_text() {
        let mut capture = QuickCapture::default();
        capture.open();
        assert_eq!(capture.draft, "");
        capture.edit("   ".into());
        assert_eq!(capture.submit(), None);
        assert_eq!(capture.draft, "   ");
        capture.edit("  Acheter du pain  ".into());
        assert_eq!(capture.submit().as_deref(), Some("Acheter du pain"));
        capture.fail("disque indisponible".into());
        assert_eq!(capture.draft, "  Acheter du pain  ");
        assert_eq!(capture.error.as_deref(), Some("disque indisponible"));
        capture.open();
        assert_eq!(capture.draft, "");
        assert_eq!(capture.error, None);
    }

    #[test]
    fn panel_navigation_skips_completed_tasks_and_stays_within_bounds() {
        let mut panel = PanelNavigation::default();
        let active = [0, 2, 4];
        panel.open(active.len());
        assert_eq!(panel.selected, Some(0));
        panel.previous();
        assert_eq!(panel.selected, Some(0));
        panel.next(active.len());
        panel.next(active.len());
        panel.next(active.len());
        assert_eq!(panel.selected_index(&active), Some(4));
        panel.previous();
        assert_eq!(panel.selected_index(&active), Some(2));
        panel.refresh(1);
        assert_eq!(panel.selected, Some(0));
        panel.refresh(0);
        assert_eq!(panel.selected, None);
        panel.next(0);
        assert_eq!(panel.selected, None);
        panel.open(2);
        assert_eq!(panel.selected, Some(0));
    }

    #[test]
    fn keyboard_selection_scrolls_only_when_its_entire_row_is_outside_view() {
        assert_eq!(scroll_offset_for_row(480.0, 30.0, 0.0, 460.0), Some(50.0));
        assert_eq!(scroll_offset_for_row(480.0, 30.0, 50.0, 460.0), None);
        assert_eq!(scroll_offset_for_row(20.0, 30.0, 50.0, 460.0), Some(20.0));
        assert_eq!(scroll_offset_for_row(20.0, 60.0, 20.0, 460.0), None);
        assert_eq!(scroll_offset_for_row(20.0, 500.0, 0.0, 460.0), Some(20.0));
    }

    #[test]
    fn panel_shows_existing_active_tasks_and_refreshes_after_external_edit_and_source_change() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.md");
        std::fs::write(&path, "- [ ] Première\n- [x] Terminée\n- [ ] Deuxième\n").unwrap();
        let mut repo = TaskRepository::open(path.clone(), false).unwrap();
        let mut snapshot = Snapshot {
            revision: 0,
            tasks: vec![],
        };
        let mut navigation = PanelNavigation::default();

        replace_panel_snapshot(&mut snapshot, &mut navigation, repo.snapshot());
        let active = panel_active_indices(&snapshot);
        assert_eq!(
            active
                .iter()
                .map(|&i| snapshot.tasks[i].title.as_str())
                .collect::<Vec<_>>(),
            ["Première", "Deuxième"]
        );
        navigation.open(active.len());
        navigation.next(active.len());
        assert_eq!(navigation.selected_index(&active), Some(2));

        std::fs::write(&path, "- [x] Première\n- [ ] Extérieure\n").unwrap();
        assert!(repo.reload().unwrap());
        replace_panel_snapshot(&mut snapshot, &mut navigation, repo.snapshot());
        let active = panel_active_indices(&snapshot);
        assert_eq!(
            active
                .iter()
                .map(|&i| snapshot.tasks[i].title.as_str())
                .collect::<Vec<_>>(),
            ["Extérieure"]
        );
        assert_eq!(navigation.selected_index(&active), Some(1));

        let other = dir.path().join("autre.md");
        std::fs::write(&other, "- [ ] Autre fichier\n- [x] Ancienne\n").unwrap();
        let candidate = TaskRepository::open(other, false).unwrap();
        replace_panel_snapshot(&mut snapshot, &mut navigation, candidate.snapshot());
        let active = panel_active_indices(&snapshot);
        assert_eq!(
            active
                .iter()
                .map(|&i| snapshot.tasks[i].title.as_str())
                .collect::<Vec<_>>(),
            ["Autre fichier"]
        );
        assert_eq!(navigation.selected_index(&active), Some(0));
    }

    #[test]
    fn capture_only_closes_on_confirmed_write_and_preserves_draft_on_conflict() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.md");
        let mut repo = TaskRepository::open(path.clone(), true).unwrap();
        let mut capture = QuickCapture::default();
        capture.open();
        capture.edit("   ".into());
        assert!(save_capture(&mut capture, &mut repo).is_none());
        assert_eq!(repo.snapshot().tasks.len(), 0);

        capture.edit("  Première tâche  ".into());
        std::fs::write(&path, "- [ ] Tâche extérieure\n").unwrap();
        assert!(save_capture(&mut capture, &mut repo).is_none());
        assert_eq!(capture.draft, "  Première tâche  ");
        assert!(capture.error.is_some());
        assert_eq!(repo.snapshot().tasks[0].title, "Tâche extérieure");

        let confirmed = save_capture(&mut capture, &mut repo).unwrap();
        assert_eq!(confirmed.tasks.len(), 2);
        assert_eq!(confirmed.tasks[1].title, "Première tâche");
        assert_eq!(capture.draft, "  Première tâche  ");
        assert!(
            std::fs::read_to_string(&path)
                .unwrap()
                .contains("- [ ] Première tâche")
        );
    }

    #[test]
    fn capture_submitted_message_keeps_window_and_draft_on_error_then_closes_on_retry() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.md");
        let mut repo = TaskRepository::open(path.clone(), true).unwrap();
        let mut snapshot = repo.snapshot();
        let mut navigation = PanelNavigation::default();
        let mut capture = QuickCapture::default();
        let id = window::Id::unique();
        capture.open();
        assert!(!update_capture(
            Message::CaptureSubmitted(id),
            Some(id),
            &mut capture,
            &mut repo,
            &mut snapshot,
            &mut navigation,
        ));
        assert!(capture.error.is_some());
        assert!(snapshot.tasks.is_empty());
        assert!(!update_capture(
            Message::CaptureChanged(id, "  Une tâche  ".into()),
            Some(id),
            &mut capture,
            &mut repo,
            &mut snapshot,
            &mut navigation,
        ));
        std::fs::write(&path, "- [ ] Externe\n").unwrap();
        assert!(!update_capture(
            Message::CaptureSubmitted(id),
            Some(id),
            &mut capture,
            &mut repo,
            &mut snapshot,
            &mut navigation,
        ));
        assert_eq!(capture.draft, "  Une tâche  ");
        assert!(capture.error.is_some());
        assert_eq!(snapshot.tasks[0].title, "Externe");
        assert!(!update_capture(
            Message::CaptureSubmitted(window::Id::unique()),
            Some(id),
            &mut capture,
            &mut repo,
            &mut snapshot,
            &mut navigation,
        ));
        assert_eq!(snapshot.tasks.len(), 1);
        assert!(update_capture(
            Message::CaptureSubmitted(id),
            Some(id),
            &mut capture,
            &mut repo,
            &mut snapshot,
            &mut navigation,
        ));
        assert_eq!(snapshot.tasks[1].title, "Une tâche");
        assert_eq!(repo.snapshot().tasks.len(), 2);
        assert_eq!(navigation.selected, Some(0));
        assert!(!update_capture(
            Message::CaptureSubmitted(id),
            None,
            &mut capture,
            &mut repo,
            &mut snapshot,
            &mut navigation,
        ));
        assert_eq!(repo.snapshot().tasks.len(), 2);
    }

    #[test]
    fn rapid_panel_toggle_before_opened_rejects_old_open_and_close() {
        let old = window::Id::unique();
        let new = window::Id::unique();
        let mut slot = None;
        assert!(window_transition(&mut slot, WindowTransition::Reserve(old)));
        assert!(!window_transition(
            &mut slot,
            WindowTransition::Reserve(new)
        ));
        assert!(window_transition(&mut slot, WindowTransition::Close(old)));
        assert_eq!(slot, None);
        assert!(window_transition(&mut slot, WindowTransition::Reserve(new)));
        assert!(!window_transition(&mut slot, WindowTransition::Opened(old)));
        assert!(!window_transition(&mut slot, WindowTransition::Close(old)));
        assert_eq!(slot, Some(new));
        assert!(window_transition(&mut slot, WindowTransition::Opened(new)));
    }

    #[test]
    fn capture_closed_before_opened_and_late_closed_never_dismisses_new_capture() {
        let old = window::Id::unique();
        let new = window::Id::unique();
        let mut slot = None;
        assert!(window_transition(&mut slot, WindowTransition::Reserve(old)));
        assert!(window_transition(&mut slot, WindowTransition::Close(old)));
        assert!(window_transition(&mut slot, WindowTransition::Reserve(new)));
        assert!(!window_transition(&mut slot, WindowTransition::Opened(old)));
        assert!(!window_transition(&mut slot, WindowTransition::Closed(old)));
        assert_eq!(slot, Some(new));
        assert!(window_transition(&mut slot, WindowTransition::Closed(new)));
        assert_eq!(slot, None);
    }
}
