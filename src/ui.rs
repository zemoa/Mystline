use crate::desktop::{self, Event};
mod keyboard_button;
mod task_store;
use chrono::{Local, NaiveDate};
use futures::{SinkExt, StreamExt, channel::mpsc::UnboundedReceiver};
use iced::{
    Element, Rectangle, Subscription, Task, Vector,
    advanced::widget::{
        self,
        operation::{self, Outcome},
    },
    event, keyboard,
    widget::{
        button, checkbox, column, container, rich_text, row, scrollable, span, text, text_input,
    },
    window,
};
use keyboard_button::keyboard_button;
use mystline::{
    application::TaskCommand,
    config::{self, Config},
    presentation::{
        input::{HELP, parse_input},
        panel::{
            DeadlineTone, Editor, PanelMode, PanelState, Section, next_day_delay,
            task_deadline_tone,
        },
    },
    repository::{Snapshot, TaskRepository},
};
use notify::RecommendedWatcher;
use std::{
    cell::RefCell,
    path::PathBuf,
    sync::{Mutex, OnceLock},
};
use task_store::{JobResult, OperationKind, SourceRevision, TaskStore};

static RECEIVER: OnceLock<Mutex<Option<UnboundedReceiver<Event>>>> = OnceLock::new();

#[derive(Default)]
struct QuickCapture {
    draft: String,
    error: Option<String>,
    help: bool,
}

impl QuickCapture {
    fn open(&mut self) {
        self.draft.clear();
        self.error = None;
        self.help = false;
    }

    fn edit(&mut self, draft: String) {
        self.draft = draft;
        self.error = None;
    }

    #[cfg(test)]
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
    // Ordinal among visible tasks in the current view, not the repository index.
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

#[cfg(test)]
fn replace_panel_snapshot(
    snapshot: &mut Snapshot,
    navigation: &mut PanelNavigation,
    panel: &PanelState,
    confirmed: Snapshot,
) {
    *snapshot = confirmed;
    navigation.refresh(panel.indices(snapshot).len());
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
    focused_row: bool,
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
        if !self.focused_row && id == Some(&widget::Id::new("panel-selected")) {
            self.row = Some(bounds);
        }
    }

    fn focusable(
        &mut self,
        _id: Option<&widget::Id>,
        bounds: Rectangle,
        state: &mut dyn operation::Focusable,
    ) {
        if self.focused_row && state.is_focused() {
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
        focused_row: false,
    })
}

fn reveal_focused() -> Task<Message> {
    widget::operate(ScrollSelected {
        viewport: None,
        row: None,
        focused_row: true,
    })
}

fn refresh_panel(state: &mut State, preferred: Option<usize>) {
    refresh_panel_parts(
        &mut state.panel_view,
        &mut state.navigation,
        &state.tasks.snapshot,
        preferred,
    );
}

fn refresh_panel_parts(
    panel_view: &mut PanelState,
    navigation: &mut PanelNavigation,
    snapshot: &Snapshot,
    preferred: Option<usize>,
) {
    panel_view.refresh(snapshot);
    let indices = panel_view.indices(snapshot);
    if let Some(ordinal) =
        preferred.and_then(|index| indices.iter().position(|&candidate| candidate == index))
    {
        navigation.selected = Some(ordinal);
    } else {
        navigation.refresh(indices.len());
    }
}

fn selected_task(state: &State) -> Option<usize> {
    state
        .navigation
        .selected_index(&state.panel_view.indices(&state.tasks.snapshot))
}

fn begin_panel_completion(
    panel: &mut PanelState,
    tasks: &mut TaskStore,
    source: SourceRevision,
    position: usize,
    today: NaiveDate,
) -> Option<task_store::Job> {
    if panel.mode != PanelMode::Daily || panel.editor.is_some() || tasks.is_busy() {
        return None;
    }
    panel.today = today;
    match tasks.begin_write(source, TaskCommand::Complete { position, today }) {
        Ok(job) => {
            panel.error = None;
            Some(job)
        }
        Err(error) => {
            panel.error = Some(error);
            None
        }
    }
}

fn change_panel_day(
    panel: &mut PanelState,
    navigation: &mut PanelNavigation,
    snapshot: &Snapshot,
    today: NaiveDate,
) {
    let selected = navigation.selected_index(&panel.indices(snapshot));
    panel.today = today;
    refresh_panel_parts(panel, navigation, snapshot, selected);
}

fn toggle_panel_history(
    panel: &mut PanelState,
    navigation: &mut PanelNavigation,
    snapshot: &Snapshot,
) {
    let selected = navigation.selected_index(&panel.indices(snapshot));
    panel.toggle_history();
    refresh_panel_parts(panel, navigation, snapshot, selected);
}

fn escape_panel_view(
    panel_view: &mut PanelState,
    navigation: &mut PanelNavigation,
    snapshot: &Snapshot,
) -> bool {
    let selected = navigation.selected_index(&panel_view.indices(snapshot));
    if !panel_view.escape() {
        return false;
    }
    refresh_panel_parts(panel_view, navigation, snapshot, selected);
    true
}

fn start_capture(
    message: Message,
    current: Option<window::Id>,
    capture: &mut QuickCapture,
    tasks: &mut TaskStore,
) -> Option<task_store::Job> {
    match message {
        Message::CaptureChanged(id, draft) if current == Some(id) => capture.edit(draft),
        Message::CaptureSubmitted(id) if current == Some(id) => {
            let result = parse_input(&capture.draft, Local::now().date_naive())
                .map_err(|error| error.to_string())
                .and_then(|input| {
                    tasks.begin_write(tasks.source_revision(), TaskCommand::Create(input))
                });
            match result {
                Ok(job) => {
                    capture.error = None;
                    return Some(job);
                }
                Err(error) => capture.fail(error),
            }
        }
        _ => (),
    }
    None
}

fn finish_capture(
    capture: &mut QuickCapture,
    current: Option<window::Id>,
    submitted: window::Id,
    draft: &str,
    result: &JobResult,
) -> bool {
    if current != Some(submitted) {
        return false;
    }
    if let Some(error) = &result.error {
        capture.fail(error.clone());
        false
    } else {
        capture.error = None;
        capture.draft == draft
    }
}

fn finish_edit(panel: &mut PanelState, mut editor: Editor, result: &JobResult) -> bool {
    if let Some(error) = &result.error {
        editor.error = Some(error.clone());
        panel.editor = Some(editor);
        true
    } else {
        false
    }
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

enum PendingWrite {
    Capture { id: window::Id, draft: String },
    Edit(mystline::presentation::panel::Editor),
    List(TaskCommand),
}

struct State {
    config_dir: PathBuf,
    config: Config,
    tasks: TaskStore,
    pending_write: Option<PendingWrite>,
    reload_pending: bool,
    quit_pending: bool,
    watcher: RecommendedWatcher,
    _tray: desktop::Tray,
    _hotkeys: Option<desktop::Shortcuts>,
    shortcut_status: (bool, bool),
    quick_capture: QuickCapture,
    navigation: PanelNavigation,
    panel_view: PanelState,
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
    Key(window::Id, PanelKey),
    PanelKey(window::Id, PanelKey, bool),
    SearchChanged(window::Id, String),
    SearchLeave(window::Id),
    FilterTag(window::Id, Option<String>),
    EditChanged(window::Id, String),
    EditSubmitted(window::Id),
    SelectTask(window::Id, usize),
    CompleteTask(window::Id, SourceRevision, usize),
    RepositoryFinished(JobResult),
    ToggleHistory(window::Id),
    DayChanged(NaiveDate),
    PathChanged(String),
    SetSource(bool),
    Autostart(bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PanelKey {
    Escape,
    Help,
    Search,
    Edit,
    Complete,
    History,
    Move(bool),
    Reorder(bool),
    Tab(bool),
}

fn keyboard_message(
    key: keyboard::Key,
    modifiers: keyboard::Modifiers,
    status: event::Status,
    id: window::Id,
) -> Option<Message> {
    use keyboard::{Key, key::Named};
    let action = match key.as_ref() {
        Key::Named(Named::Escape) => PanelKey::Escape,
        Key::Named(Named::F1) => PanelKey::Help,
        Key::Character(c)
            if modifiers == keyboard::Modifiers::CTRL && c.eq_ignore_ascii_case("f") =>
        {
            PanelKey::Search
        }
        Key::Character(c)
            if modifiers == keyboard::Modifiers::CTRL && c.eq_ignore_ascii_case("h") =>
        {
            PanelKey::History
        }
        Key::Named(Named::Tab) if !modifiers.control() && !modifiers.alt() && !modifiers.logo() => {
            PanelKey::Tab(modifiers.shift())
        }
        _ if status == event::Status::Captured => return None,
        Key::Named(Named::Enter) if modifiers.is_empty() => PanelKey::Edit,
        Key::Named(Named::Space) if modifiers.is_empty() => PanelKey::Complete,
        Key::Named(Named::ArrowDown) if modifiers.is_empty() => PanelKey::Move(true),
        Key::Named(Named::ArrowUp) if modifiers.is_empty() => PanelKey::Move(false),
        Key::Named(Named::ArrowDown) if modifiers == keyboard::Modifiers::CTRL => {
            PanelKey::Reorder(true)
        }
        Key::Named(Named::ArrowUp) if modifiers == keyboard::Modifiers::CTRL => {
            PanelKey::Reorder(false)
        }
        _ => return None,
    };
    Some(Message::Key(id, action))
}

// Unlike is_focused(id), this operation also replies when no text field exists.
// Restrict Tab to this panel's controls when capture/settings are also open.
struct PanelTab {
    ids: Vec<widget::Id>,
    focused: Option<usize>,
    previous: bool,
}

impl widget::Operation<Message> for PanelTab {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn widget::Operation<Message>)) {
        operate(self);
    }
    fn focusable(
        &mut self,
        id: Option<&widget::Id>,
        _bounds: Rectangle,
        state: &mut dyn operation::Focusable,
    ) {
        if state.is_focused() {
            self.focused = id
                .and_then(|id| self.ids.iter().position(|candidate| candidate == id))
                .or(self.focused);
        }
    }
    fn finish(&self) -> Outcome<Message> {
        let target = match (self.focused, self.previous) {
            (None, false) => self.ids.first(),
            (None, true) => self.ids.last(),
            (Some(index), true) => index.checked_sub(1).and_then(|index| self.ids.get(index)),
            (Some(index), false) => self.ids.get(index + 1),
        };
        match target {
            Some(id) => Outcome::Chain(Box::new(operation::focusable::focus(id.clone()))),
            None => Outcome::Chain(Box::new(operation::focusable::unfocus())),
        }
    }
}

fn panel_tab(state: &State, previous: bool) -> Task<Message> {
    let mut ids = Vec::new();
    if state.panel_view.editor.is_some() {
        ids.push(widget::Id::new("panel-edit"));
    } else {
        ids.push(widget::Id::new("panel-history"));
        ids.push(widget::Id::new("panel-find"));
        if state.panel_view.search_active || !state.panel_view.query.is_empty() {
            ids.push(widget::Id::new("panel-search"));
        }
        if state.panel_view.tag.is_some() {
            ids.push(widget::Id::new("panel-clear-tag"));
        }
        for row in state.panel_view.rows(&state.tasks.snapshot) {
            for position in 0..state.tasks.snapshot.tasks[row.index].tags.len() {
                ids.push(format!("panel-tag-{}-{position}", row.index).into());
            }
        }
    }
    widget::operate(PanelTab {
        ids,
        focused: None,
        previous,
    })
}

struct TextFocus {
    id: window::Id,
    key: PanelKey,
    focused: bool,
}
impl widget::Operation<Message> for TextFocus {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn widget::Operation<Message>)) {
        operate(self);
    }
    fn focusable(
        &mut self,
        id: Option<&widget::Id>,
        _bounds: Rectangle,
        state: &mut dyn operation::Focusable,
    ) {
        if id == Some(&widget::Id::new("panel-search"))
            || id == Some(&widget::Id::new("panel-edit"))
        {
            self.focused |= state.is_focused();
        }
    }
    fn finish(&self) -> Outcome<Message> {
        Outcome::Some(Message::PanelKey(self.id, self.key, self.focused))
    }
}

fn day_stream() -> impl futures::Stream<Item = Message> {
    iced::stream::channel(1, async |mut output| {
        loop {
            tokio::time::sleep(next_day_delay(Local::now())).await;
            if output
                .send(Message::DayChanged(Local::now().date_naive()))
                .await
                .is_err()
            {
                break;
            }
        }
    })
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
        tasks: TaskStore::new(repo),
        pending_write: None,
        reload_pending: false,
        quit_pending: false,
        watcher,
        _tray: tray,
        _hotkeys: hotkeys,
        shortcut_status,
        quick_capture: QuickCapture::default(),
        navigation: PanelNavigation::default(),
        panel_view: PanelState::new(Local::now().date_naive()),
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
        Subscription::run(day_stream),
        event::listen_with(|ev, status, id| match ev {
            iced::Event::Window(window::Event::CloseRequested) => Some(Message::Close(id)),
            iced::Event::Window(window::Event::Closed) => Some(Message::Closed(id)),
            iced::Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                keyboard_message(key, modifiers, status, id)
            }
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
        state.panel_view.today = Local::now().date_naive();
        state.panel_view.mode = PanelMode::Daily;
        state
            .navigation
            .open(state.panel_view.indices(&state.tasks.snapshot).len());
    }
    let settings = window::Settings {
        size: match surface {
            Surface::Capture => iced::Size::new(430.0, 125.0),
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

fn capture_size(state: &State, id: window::Id) -> Task<Message> {
    let height = 125.0
        + if state.quick_capture.help { 190.0 } else { 0.0 }
        + if state.quick_capture.error.is_some() {
            90.0
        } else {
            0.0
        };
    window::resize(id, iced::Size::new(430.0, height))
}

fn close(state: &mut State, id: window::Id) -> Task<Message> {
    for slot in [&mut state.capture, &mut state.panel, &mut state.settings] {
        if window_transition(slot, WindowTransition::Close(id)) {
            return window::close(id);
        }
    }
    Task::none()
}

fn write_tasks(
    state: &mut State,
    source: SourceRevision,
    command: TaskCommand,
    pending: PendingWrite,
    preferred: Option<usize>,
) -> Task<Message> {
    match state.tasks.begin_write(source, command) {
        Ok(job) => {
            state.pending_write = Some(pending);
            state.panel_view.error = None;
            refresh_panel(state, preferred);
            Task::batch([
                Task::perform(job.perform(), Message::RepositoryFinished),
                reveal_selected(),
            ])
        }
        Err(error) => {
            match pending {
                PendingWrite::Capture { .. } => state.quick_capture.fail(error),
                PendingWrite::Edit(_) => {
                    if let Some(editor) = &mut state.panel_view.editor {
                        editor.error = Some(error);
                    }
                }
                PendingWrite::List(_) => state.panel_view.error = Some(error),
            }
            Task::none()
        }
    }
}

fn reload_tasks(state: &mut State) -> Task<Message> {
    match state.tasks.begin_reload() {
        Some(job) => {
            state.reload_pending = false;
            Task::perform(job.perform(), Message::RepositoryFinished)
        }
        None => {
            state.reload_pending = true;
            Task::none()
        }
    }
}

fn repository_finished(state: &mut State, result: JobResult) -> Task<Message> {
    let mut selected = selected_task(state);
    if !state.tasks.finish(&result) {
        return Task::none();
    }
    let mut effect = Task::none();
    if result.kind == OperationKind::Write {
        match state.pending_write.take() {
            Some(PendingWrite::Capture { id, draft }) => {
                if finish_capture(&mut state.quick_capture, state.capture, id, &draft, &result) {
                    effect = close(state, id);
                } else if let Some(error) = &result.error {
                    if state.capture == Some(id) {
                        effect = capture_size(state, id).chain(capture_focus(id));
                    } else {
                        state.panel_view.error =
                            Some(format!("{error}\nTexte non enregistré : {draft}"));
                    }
                }
            }
            Some(PendingWrite::Edit(editor)) => {
                if finish_edit(&mut state.panel_view, editor, &result) && state.panel.is_some() {
                    effect = iced::widget::operation::focus("panel-edit");
                }
            }
            Some(PendingWrite::List(command)) => {
                state.panel_view.error = result.error.clone();
                if result.error.is_some()
                    && let TaskCommand::Reorder { from, to } = command
                {
                    selected = selected.map(|index| {
                        if index == from {
                            to
                        } else if index == to {
                            from
                        } else {
                            index
                        }
                    });
                }
            }
            None => (),
        }
    } else {
        state.error = result.error;
    }
    refresh_panel(state, selected);
    if state.quit_pending {
        return iced::exit();
    }
    effect = effect.chain(reveal_selected());
    if state.reload_pending {
        effect = effect.chain(reload_tasks(state));
    }
    effect
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
        Message::Desktop(Event::Quit) => {
            if state.tasks.is_busy() {
                state.quit_pending = true;
                Task::none()
            } else {
                iced::exit()
            }
        }
        Message::Desktop(Event::ShortcutStatus(a, b)) => {
            state.shortcut_status = (a, b);
            Task::none()
        }
        Message::Desktop(Event::FileChanged) => {
            desktop::acknowledge_file_change();
            reload_tasks(state)
        }
        Message::RepositoryFinished(result) => repository_finished(state, result),
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
        Message::Key(id, key) => {
            if state.capture == Some(id) {
                match key {
                    PanelKey::Help => state.quick_capture.help = !state.quick_capture.help,
                    PanelKey::Escape if state.quick_capture.help => {
                        state.quick_capture.help = false
                    }
                    PanelKey::Escape => return close(state, id),
                    _ => return Task::none(),
                }
                return capture_size(state, id).chain(capture_focus(id));
            }
            if state.panel == Some(id) {
                return widget::operate(TextFocus {
                    id,
                    key,
                    focused: false,
                });
            }
            if key == PanelKey::Escape {
                return close(state, id);
            }
            Task::none()
        }
        Message::PanelKey(id, key, text_focused) if state.panel == Some(id) => {
            match key {
                PanelKey::Escape => {
                    if !escape_panel_view(
                        &mut state.panel_view,
                        &mut state.navigation,
                        &state.tasks.snapshot,
                    ) {
                        return close(state, id);
                    }
                    if state.panel_view.editor.is_some() {
                        return iced::widget::operation::focus("panel-edit");
                    }
                    return widget::operate(operation::focusable::unfocus())
                        .chain(reveal_selected());
                }
                PanelKey::Help => {
                    if let Some(editor) = &mut state.panel_view.editor {
                        editor.help = !editor.help;
                        return iced::widget::operation::focus("panel-edit");
                    }
                }
                PanelKey::Search if state.panel_view.editor.is_none() => {
                    state.panel_view.search_active = true;
                    return iced::widget::operation::focus("panel-search");
                }
                PanelKey::History if state.panel_view.editor.is_none() => {
                    return update(state, Message::ToggleHistory(id));
                }
                PanelKey::Complete if !text_focused && state.panel_view.editor.is_none() => {
                    if let Some(index) = selected_task(state)
                        && !state.tasks.snapshot.tasks[index].completed
                    {
                        return update(
                            state,
                            Message::CompleteTask(id, state.tasks.source_revision(), index),
                        );
                    }
                }
                PanelKey::Tab(previous) => {
                    return panel_tab(state, previous).chain(reveal_focused());
                }
                PanelKey::Edit
                    if !text_focused
                        && state.panel_view.editor.is_none()
                        && !state.tasks.is_busy() =>
                {
                    if let Some(index) = selected_task(state) {
                        state.panel_view.begin_edit(&state.tasks.snapshot, index);
                        if state.panel_view.editor.is_some() {
                            return iced::widget::operation::focus("panel-edit");
                        }
                    }
                }
                PanelKey::Move(down) if !text_focused && state.panel_view.editor.is_none() => {
                    return update(state, Message::PanelMove(id, down));
                }
                PanelKey::Reorder(down) if !text_focused && state.panel_view.editor.is_none() => {
                    let selected = selected_task(state);
                    state.panel_view.today = Local::now().date_naive();
                    refresh_panel(state, selected);
                    if let Some(from) = selected_task(state)
                        && let Some(to) =
                            state.panel_view.neighbor(&state.tasks.snapshot, from, down)
                    {
                        let command = TaskCommand::Reorder { from, to };
                        return write_tasks(
                            state,
                            state.tasks.source_revision(),
                            command.clone(),
                            PendingWrite::List(command),
                            Some(to),
                        );
                    }
                }
                _ => (),
            }
            Task::none()
        }
        Message::PanelKey(..) => Task::none(),
        Message::SearchChanged(id, query)
            if state.panel == Some(id) && state.panel_view.editor.is_none() =>
        {
            let selected = selected_task(state);
            state.panel_view.query = query;
            refresh_panel(state, selected);
            reveal_selected()
        }
        Message::SearchLeave(id) if state.panel == Some(id) => {
            widget::operate(operation::focusable::unfocus()).chain(reveal_selected())
        }
        Message::FilterTag(id, tag)
            if state.panel == Some(id) && state.panel_view.editor.is_none() =>
        {
            let selected = selected_task(state);
            state.panel_view.tag = tag;
            refresh_panel(state, selected);
            widget::operate(operation::focusable::unfocus()).chain(reveal_selected())
        }
        Message::EditChanged(id, draft) if state.panel == Some(id) => {
            if let Some(editor) = &mut state.panel_view.editor {
                editor.draft = draft;
                if !editor.invalidated {
                    editor.error = None;
                }
            }
            Task::none()
        }
        Message::EditSubmitted(id) if state.panel == Some(id) => {
            let Some(editor) = &state.panel_view.editor else {
                return Task::none();
            };
            if editor.invalidated {
                return Task::none();
            }
            let input = match parse_input(&editor.draft, Local::now().date_naive()) {
                Ok(input) => input,
                Err(error) => {
                    state.panel_view.editor.as_mut().unwrap().error = Some(error.to_string());
                    return Task::none();
                }
            };
            let position = editor.position;
            let editor = editor.clone();
            let mut source = state.tasks.source_revision();
            source = source.with_revision(editor.revision);
            let effect = write_tasks(
                state,
                source,
                TaskCommand::Update { position, input },
                PendingWrite::Edit(editor),
                Some(position),
            );
            if matches!(state.pending_write, Some(PendingWrite::Edit(_))) {
                state.panel_view.editor = None;
                return Task::batch([effect, widget::operate(operation::focusable::unfocus())]);
            }
            effect
        }
        Message::SelectTask(id, index)
            if state.panel == Some(id) && state.panel_view.editor.is_none() =>
        {
            let indices = state.panel_view.indices(&state.tasks.snapshot);
            state.navigation.selected = indices.iter().position(|&candidate| candidate == index);
            widget::operate(operation::focusable::unfocus()).chain(reveal_selected())
        }
        Message::CompleteTask(id, source, index) if state.panel == Some(id) => {
            let today = Local::now().date_naive();
            if let Some(job) = begin_panel_completion(
                &mut state.panel_view,
                &mut state.tasks,
                source,
                index,
                today,
            ) {
                state.pending_write = Some(PendingWrite::List(TaskCommand::Complete {
                    position: index,
                    today,
                }));
                refresh_panel(state, Some(index));
                Task::batch([
                    Task::perform(job.perform(), Message::RepositoryFinished),
                    reveal_selected(),
                ])
            } else {
                Task::none()
            }
        }
        Message::ToggleHistory(id)
            if state.panel == Some(id) && state.panel_view.editor.is_none() =>
        {
            toggle_panel_history(
                &mut state.panel_view,
                &mut state.navigation,
                &state.tasks.snapshot,
            );
            widget::operate(operation::focusable::unfocus()).chain(reveal_selected())
        }
        Message::SearchChanged(..)
        | Message::SearchLeave(..)
        | Message::FilterTag(..)
        | Message::EditChanged(..)
        | Message::EditSubmitted(..)
        | Message::SelectTask(..)
        | Message::CompleteTask(..)
        | Message::ToggleHistory(..) => Task::none(),
        Message::DayChanged(today) => {
            change_panel_day(
                &mut state.panel_view,
                &mut state.navigation,
                &state.tasks.snapshot,
                today,
            );
            reveal_selected()
        }
        Message::CaptureChanged(id, draft) if state.capture == Some(id) => {
            if matches!(&state.pending_write, Some(PendingWrite::Capture { id: pending_id, .. }) if *pending_id == id)
            {
                return Task::none();
            }
            let had_error = state.quick_capture.error.is_some();
            state.quick_capture.edit(draft);
            if had_error {
                capture_size(state, id)
            } else {
                Task::none()
            }
        }
        Message::CaptureSubmitted(id) if state.capture == Some(id) => {
            if matches!(&state.pending_write, Some(PendingWrite::Capture { id: pending_id, .. }) if *pending_id == id)
            {
                return Task::none();
            }
            let position = state.tasks.snapshot.tasks.len();
            if let Some(job) = start_capture(
                Message::CaptureSubmitted(id),
                state.capture,
                &mut state.quick_capture,
                &mut state.tasks,
            ) {
                state.pending_write = Some(PendingWrite::Capture {
                    id,
                    draft: state.quick_capture.draft.clone(),
                });
                refresh_panel(state, Some(position));
                Task::batch([
                    Task::perform(job.perform(), Message::RepositoryFinished),
                    reveal_selected(),
                ])
            } else {
                capture_size(state, id)
            }
        }
        Message::CaptureChanged(..) | Message::CaptureSubmitted(..) => Task::none(),
        Message::PanelMove(id, down) => {
            if state.panel == Some(id) && state.navigation.focused {
                if down {
                    state
                        .navigation
                        .next(state.panel_view.indices(&state.tasks.snapshot).len());
                } else {
                    state.navigation.previous();
                }
                return widget::operate(operation::focusable::unfocus()).chain(reveal_selected());
            }
            Task::none()
        }
        Message::PanelFocus(id, focused) => {
            if state.panel == Some(id) {
                state.navigation.focused = focused;
                if focused {
                    let selected = selected_task(state);
                    state.panel_view.today = Local::now().date_naive();
                    refresh_panel(state, selected);
                    return reveal_selected();
                }
            }
            Task::none()
        }
        Message::PathChanged(path) => {
            state.draft_path = path;
            Task::none()
        }
        Message::SetSource(create) => {
            if state.tasks.is_busy() {
                state.error = Some("Attendre la fin de l'enregistrement ou du rechargement avant de changer de fichier.".into());
                return Task::none();
            }
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
                state.tasks.replace_source(candidate);
                state.panel_view.invalidate_edit();
                refresh_panel(state, None);
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
        capture = capture.push(text("Entrée : ajouter · F1 : aide · Échap : fermer").size(12));
        if state.quick_capture.help {
            capture = capture.push(text(HELP).size(13));
        }
        if let Some(error) = &state.quick_capture.error {
            capture = capture.push(scrollable(text(error)).height(75));
        }
        capture.into()
    } else if state.panel == Some(id) {
        panel_view(state, id)
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

fn panel_view(state: &State, id: window::Id) -> Element<'_, Message> {
    panel_content(
        &state.panel_view,
        &state.navigation,
        &state.tasks.snapshot,
        state.tasks.source_revision(),
        id,
    )
}

fn panel_content<'a>(
    panel: &'a PanelState,
    navigation: &PanelNavigation,
    snapshot: &'a Snapshot,
    source: SourceRevision,
    id: window::Id,
) -> Element<'a, Message> {
    if let Some(editor) = &panel.editor {
        let mut form = column![
            text("Modifier la tâche").size(22),
            text_input("Titre /p date /d date #tag", &editor.draft)
                .id("panel-edit")
                .on_input(move |draft| Message::EditChanged(id, draft))
                .on_submit(Message::EditSubmitted(id)),
            text("Entrée : enregistrer · Échap : annuler · F1 : aide").size(12),
        ]
        .spacing(12);
        if editor.help {
            form = form.push(text(HELP).size(13));
        }
        if let Some(error) = &editor.error {
            form = form.push(text(error).color(iced::Color::from_rgb8(180, 45, 45)));
        }
        return scrollable(form).into();
    }
    let rows = panel.rows(snapshot);
    let selected = navigation.selected_index(&rows.iter().map(|row| row.index).collect::<Vec<_>>());
    let mut header = column![
        row![
            text(if panel.mode == PanelMode::History {
                "Historique"
            } else {
                "Mes tâches"
            })
            .size(24),
            keyboard_button(
                "panel-history",
                if panel.mode == PanelMode::History {
                    "Retour"
                } else {
                    "Historique"
                }
                .into(),
                Message::ToggleHistory(id)
            ),
            keyboard_button(
                "panel-find",
                "Rechercher".into(),
                Message::Key(id, PanelKey::Search)
            ),
        ]
        .spacing(10)
        .wrap()
    ]
    .spacing(10);
    if panel.search_active || !panel.query.is_empty() {
        header = header.push(
            text_input("Rechercher un titre ou un tag", &panel.query)
                .id("panel-search")
                .on_input(move |query| Message::SearchChanged(id, query))
                .on_submit(Message::SearchLeave(id)),
        );
    }
    if let Some(tag) = &panel.tag {
        header = header.push(keyboard_button(
            "panel-clear-tag",
            format!("#{tag} · Retirer le filtre"),
            Message::FilterTag(id, None),
        ));
    }
    if let Some(error) = &panel.error {
        header = header.push(text(error).color(iced::Color::from_rgb8(180, 45, 45)));
    }
    let mut list = column![].spacing(14);
    let sections: &[Section] = if panel.mode == PanelMode::History {
        &[Section::All]
    } else {
        &Section::ALL
    };
    for &section in sections {
        let mut section_title = text(if panel.mode == PanelMode::History {
            "Tâches terminées"
        } else {
            section.label()
        })
        .size(18);
        if section == Section::Overdue {
            section_title = section_title.color(iced::Color::from_rgb8(180, 45, 45));
        }
        list = list.push(section_title);
        let mut count = 0;
        for entry in rows.iter().filter(|entry| entry.section == section) {
            count += 1;
            let task = &snapshot.tasks[entry.index];
            let index = entry.index;
            let marker = if selected == Some(index) { "▸" } else { " " };
            let mut completion = checkbox(task.completed);
            if !task.completed && panel.mode == PanelMode::Daily {
                completion =
                    completion.on_toggle(move |_| Message::CompleteTask(id, source, index));
            }
            let title = rich_text![
                span::<(), _>(format!("{marker} {}", task.title)).strikethrough(task.completed)
            ];
            let mut content = column![
                row![
                    container(completion).id(format!("panel-complete-{index}")),
                    button(title)
                        .style(button::text)
                        .width(iced::Length::Fill)
                        .on_press(Message::SelectTask(id, index))
                ]
                .spacing(8)
            ]
            .spacing(5);
            if !task.tags.is_empty() {
                let mut tags = iced::widget::Row::new().spacing(5);
                for (position, tag) in task.tags.iter().enumerate() {
                    tags = tags.push(keyboard_button(
                        format!("panel-tag-{index}-{position}"),
                        format!("#{tag}"),
                        Message::FilterTag(id, Some(tag.clone())),
                    ));
                }
                content = content.push(tags.wrap());
            }
            if let Some(date) = task.planned {
                content = content.push(text(format!("Prévu : {date}")).size(12));
            }
            if let Some(date) = task.deadline {
                let mut label = text(format!("Échéance : {date}")).size(12);
                match task_deadline_tone(task, panel.today) {
                    Some(DeadlineTone::Overdue) => {
                        label = label.color(iced::Color::from_rgb8(180, 45, 45))
                    }
                    Some(DeadlineTone::Today) => {
                        label = label.color(iced::Color::from_rgb8(160, 95, 15)).size(14)
                    }
                    _ => (),
                }
                content = content.push(label);
            }
            if panel.mode == PanelMode::History
                && let Some(date) = task.effective_completion_date()
            {
                content = content.push(text(format!("Terminée le : {date}")).size(12));
            }
            let mut item = container(content).padding(8).width(iced::Length::Fill);
            if selected == Some(index) {
                item = item.id("panel-selected").style(container::bordered_box);
            }
            list = list.push(item);
        }
        if count == 0 {
            list = list.push(
                text(if panel.mode == PanelMode::History {
                    "Aucune tâche terminée"
                } else {
                    "Aucune tâche"
                })
                .size(12),
            );
        }
    }
    if rows.is_empty() && (!panel.query.is_empty() || panel.tag.is_some()) {
        list = list.push(text(
            "Aucune tâche ne correspond à la recherche ou au filtre",
        ));
    }
    column![header, scrollable(list).id("panel-list").height(iced::Length::Fill),
        text(if panel.mode == PanelMode::History {
            "↑ ↓ : sélectionner · Ctrl+F : rechercher · Ctrl+H / Échap : retour\nTab puis Entrée : activer un bouton ou filtrer un tag"
        } else {
            "↑ ↓ : sélectionner · Espace : terminer · Entrée : éditer\nCtrl+F : rechercher · Ctrl+H : historique · Ctrl+↑ ↓ : déplacer\nTab puis Entrée : activer un bouton ou filtrer un tag"
        }).size(11),
    ].spacing(12).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f3_date(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, day).unwrap()
    }

    struct ControlBounds {
        id: widget::Id,
        bounds: Option<Rectangle>,
    }

    impl widget::Operation<Message> for ControlBounds {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn widget::Operation<Message>)) {
            operate(self);
        }
        fn container(&mut self, id: Option<&widget::Id>, bounds: Rectangle) {
            if id == Some(&self.id) {
                self.bounds = Some(bounds);
            }
        }
    }

    #[test]
    fn cliquer_la_case_termine_et_reclasse_la_tache_en_conservant_la_selection() {
        // RG-F3-02/03/08 — Étant donné le vrai panneau Iced et une tâche en retard.
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("tasks.md");
        std::fs::write(&path, "- [ ] Support @deadline(2026-10-08)\n- [ ] Autre\n").unwrap();
        let repo = TaskRepository::open_at(path.clone(), false, f3_date(9)).unwrap();
        let mut tasks = TaskStore::new(repo);
        let mut snapshot = tasks.snapshot.clone();
        let mut panel = PanelState::new(f3_date(9));
        let mut navigation = PanelNavigation::default();
        navigation.open(panel.indices(&snapshot).len());
        let id = window::Id::unique();
        let mut element = panel_content(
            &panel,
            &navigation,
            &snapshot,
            SourceRevision::for_test(snapshot.revision),
            id,
        );
        let renderer = iced::Renderer::new(iced::Font::DEFAULT, iced::Pixels(16.0));
        let mut tree = widget::Tree::new(&element);
        let viewport = Rectangle::with_size(iced::Size::new(430.0, 550.0));
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &iced::advanced::layout::Limits::new(iced::Size::ZERO, viewport.size()),
        );
        let layout = iced::advanced::Layout::new(&node);
        let mut bounds = ControlBounds {
            id: widget::Id::new("panel-complete-0"),
            bounds: None,
        };
        element.as_widget_mut().operate(
            &mut tree,
            layout,
            &renderer,
            &mut operation::black_box(&mut bounds),
        );
        let bounds = bounds
            .bounds
            .expect("la case de la tâche est rendue dans le panneau");
        assert!(viewport.contains(bounds.center()));
        // Quand la souris presse la case réellement rendue.
        let mut messages = Vec::new();
        let mut shell = iced::advanced::Shell::new(&mut messages);
        element.as_widget_mut().update(
            &mut tree,
            &iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
            layout,
            iced::mouse::Cursor::Available(bounds.center()),
            &renderer,
            &mut iced::advanced::clipboard::Null,
            &mut shell,
            &viewport,
        );
        assert!(shell.is_event_captured());
        drop(element);
        assert_eq!(messages.len(), 1);
        let Message::CompleteTask(window, revision, position) = messages.pop().unwrap() else {
            panic!("le clic doit produire la commande de complétion");
        };
        assert_eq!(window, id);
        let job =
            begin_panel_completion(&mut panel, &mut tasks, revision, position, f3_date(9)).unwrap();
        snapshot = tasks.snapshot.clone();
        refresh_panel_parts(&mut panel, &mut navigation, &snapshot, Some(position));
        assert!(snapshot.tasks[position].completed);
        let result = job.run();
        assert!(result.error.is_none());
        tasks.finish(&result);
        snapshot = tasks.snapshot.clone();
        // Alors le résultat est persisté, reclassé et toujours sélectionné dans Toutes les tâches.
        assert!(snapshot.tasks[0].completed);
        assert_eq!(panel.rows(&snapshot)[0].section, Section::All);
        assert_eq!(
            navigation.selected_index(&panel.indices(&snapshot)),
            Some(0)
        );
        assert!(panel.error.is_none());
        assert!(
            std::fs::read_to_string(&path)
                .unwrap()
                .contains("@completed(2026-10-09)")
        );
    }

    #[test]
    fn espace_termine_la_selection_et_une_activation_capturee_nest_pas_rejouee() {
        // RG-F3-01/08 — Étant donné une tâche sélectionnée dans la liste.
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("tasks.md");
        std::fs::write(&path, "- [ ] Première\n- [ ] Sélectionnée\n").unwrap();
        let repo = TaskRepository::open_at(path.clone(), false, f3_date(9)).unwrap();
        let mut tasks = TaskStore::new(repo);
        let mut snapshot = tasks.snapshot.clone();
        let mut panel = PanelState::new(f3_date(9));
        let mut navigation = PanelNavigation::default();
        navigation.open(2);
        navigation.next(2);
        let id = window::Id::unique();
        // Quand Espace non capturé est routé vers la sélection.
        let key = keyboard::Key::Named(keyboard::key::Named::Space);
        assert!(
            matches!(keyboard_message(key.clone(), keyboard::Modifiers::empty(), event::Status::Ignored, id), Some(Message::Key(window, PanelKey::Complete)) if window == id)
        );
        let index = navigation
            .selected_index(&panel.indices(&snapshot))
            .unwrap();
        let revision = snapshot.revision;
        let tasks_source = tasks.source_revision();
        let job = begin_panel_completion(&mut panel, &mut tasks, tasks_source, index, f3_date(9))
            .unwrap();
        snapshot = tasks.snapshot.clone();
        refresh_panel_parts(&mut panel, &mut navigation, &snapshot, Some(index));
        assert!(snapshot.tasks[index].completed);
        let result = job.run();
        assert!(result.error.is_none());
        tasks.finish(&result);
        snapshot = tasks.snapshot.clone();
        // Alors seule la sélection est cochée, et une touche déjà activée par un widget est ignorée.
        assert!(!snapshot.tasks[0].completed);
        assert!(snapshot.tasks[1].completed);
        assert_eq!(
            navigation.selected_index(&panel.indices(&snapshot)),
            Some(1)
        );
        assert!(
            keyboard_message(
                key,
                keyboard::Modifiers::empty(),
                event::Status::Captured,
                id
            )
            .is_none()
        );
        assert_eq!(tasks.snapshot.revision, revision + 1);
    }

    #[test]
    fn espace_dans_les_champs_saisit_du_texte_sans_cocher_une_tache() {
        // RG-F3-08 — Étant donné chaque champ réel de capture, recherche ou édition focalisé.
        for name in ["capture-title", "panel-search", "panel-edit"] {
            let id = window::Id::unique();
            let mut element: Element<'_, Message> = text_input("Saisir", "Support")
                .id(name)
                .on_input(move |draft| Message::SearchChanged(id, draft))
                .into();
            let renderer = iced::Renderer::new(iced::Font::DEFAULT, iced::Pixels(16.0));
            let mut tree = widget::Tree::new(&element);
            let viewport = Rectangle::with_size(iced::Size::new(430.0, 550.0));
            let node = element.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &iced::advanced::layout::Limits::new(iced::Size::ZERO, viewport.size()),
            );
            let layout = iced::advanced::Layout::new(&node);
            operate_in_test(
                &mut element,
                &mut tree,
                layout,
                &renderer,
                Box::new(operation::focusable::focus(widget::Id::new(name))),
            );
            let mut event = press(keyboard::key::Named::Space);
            if let iced::Event::Keyboard(keyboard::Event::KeyPressed { text, .. }) = &mut event {
                *text = Some(" ".into());
            }
            // Quand Espace est envoyé au champ.
            let mut messages = Vec::new();
            let mut shell = iced::advanced::Shell::new(&mut messages);
            element.as_widget_mut().update(
                &mut tree,
                &event,
                layout,
                iced::mouse::Cursor::Unavailable,
                &renderer,
                &mut iced::advanced::clipboard::Null,
                &mut shell,
                &viewport,
            );
            // Alors un espace est saisi et aucun raccourci de complétion n'est émis.
            assert!(shell.is_event_captured());
            assert!(
                matches!(messages.as_slice(), [Message::SearchChanged(_, draft)] if draft.contains(' ') && draft.replace(' ', "") == "Support")
            );
            assert!(
                keyboard_message(
                    keyboard::Key::Named(keyboard::key::Named::Space),
                    keyboard::Modifiers::empty(),
                    event::Status::Captured,
                    id
                )
                .is_none()
            );
        }
    }

    #[test]
    fn tab_et_entree_ouvrent_lhistorique_puis_les_fleches_et_echap_le_parcourent() {
        // RG-F3-05/08 — Étant donné le vrai bouton d'historique du panneau.
        let id = window::Id::unique();
        let snapshot = Snapshot {
            revision: 0,
            tasks: mystline::repository::codec::parse(
                "- [x] Ancienne @completed(2026-10-08)\n- [x] Aujourd'hui @completed(2026-10-09)\n",
                f3_date(9),
            )
            .unwrap(),
        };
        let mut panel = PanelState::new(f3_date(9));
        let mut navigation = PanelNavigation::default();
        navigation.open(panel.indices(&snapshot).len());
        let mut element = panel_content(
            &panel,
            &navigation,
            &snapshot,
            SourceRevision::for_test(snapshot.revision),
            id,
        );
        let renderer = iced::Renderer::new(iced::Font::DEFAULT, iced::Pixels(16.0));
        let mut tree = widget::Tree::new(&element);
        let viewport = Rectangle::with_size(iced::Size::new(430.0, 550.0));
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &iced::advanced::layout::Limits::new(iced::Size::ZERO, viewport.size()),
        );
        let layout = iced::advanced::Layout::new(&node);
        // Quand Tab focalise Historique puis Entrée l'active.
        operate_in_test(
            &mut element,
            &mut tree,
            layout,
            &renderer,
            Box::new(PanelTab {
                ids: vec![
                    widget::Id::new("panel-history"),
                    widget::Id::new("panel-find"),
                ],
                focused: None,
                previous: false,
            }),
        );
        let mut messages = Vec::new();
        let mut shell = iced::advanced::Shell::new(&mut messages);
        element.as_widget_mut().update(
            &mut tree,
            &press(keyboard::key::Named::Enter),
            layout,
            iced::mouse::Cursor::Unavailable,
            &renderer,
            &mut iced::advanced::clipboard::Null,
            &mut shell,
            &viewport,
        );
        assert!(shell.is_event_captured());
        assert!(matches!(messages.as_slice(), [Message::ToggleHistory(window)] if *window == id));
        drop(element);
        toggle_panel_history(&mut panel, &mut navigation, &snapshot);
        // Alors les tâches sont parcourables du plus récent au plus ancien, puis Échap revient au quotidien.
        assert_eq!(panel.mode, PanelMode::History);
        assert_eq!(
            navigation.selected_index(&panel.indices(&snapshot)),
            Some(1)
        );
        navigation.next(panel.indices(&snapshot).len());
        assert_eq!(
            navigation.selected_index(&panel.indices(&snapshot)),
            Some(0)
        );
        assert!(escape_panel_view(&mut panel, &mut navigation, &snapshot));
        assert_eq!(panel.mode, PanelMode::Daily);
        assert_eq!(
            navigation.selected_index(&panel.indices(&snapshot)),
            Some(1)
        );
        assert!(
            matches!(keyboard_message(keyboard::Key::Character("h".into()), keyboard::Modifiers::CTRL, event::Status::Captured, id), Some(Message::Key(window, PanelKey::History)) if window == id)
        );
    }

    #[test]
    fn le_message_de_changement_de_jour_retire_la_terminee_et_repare_la_selection() {
        // RG-F3-04 — Étant donné la dernière ligne sélectionnée, terminée aujourd'hui.
        let snapshot = Snapshot {
            revision: 1,
            tasks: mystline::repository::codec::parse(
                "- [ ] Active\n- [x] Terminée @completed(2026-10-09)\n",
                f3_date(9),
            )
            .unwrap(),
        };
        let mut panel = PanelState::new(f3_date(9));
        let mut navigation = PanelNavigation::default();
        navigation.open(2);
        navigation.next(2);
        // Quand la transition utilisée par DayChanged reçoit samedi.
        change_panel_day(&mut panel, &mut navigation, &snapshot, f3_date(10));
        // Alors la terminée disparaît et la sélection revient à la ligne encore visible.
        assert_eq!(panel.indices(&snapshot), [0]);
        assert_eq!(
            navigation.selected_index(&panel.indices(&snapshot)),
            Some(0)
        );
        assert!(snapshot.tasks[1].completed);
        assert_eq!(snapshot.revision, 1);
        toggle_panel_history(&mut panel, &mut navigation, &snapshot);
        assert_eq!(
            navigation.selected_index(&panel.indices(&snapshot)),
            Some(1)
        );
    }

    #[test]
    fn une_completion_refusee_restaure_le_snapshot_et_un_brouillon_interdit_le_cochage() {
        // RG-F3-02 — Étant donné une complétion optimiste et une modification extérieure.
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("tasks.md");
        std::fs::write(&path, "- [ ] Originale\n").unwrap();
        let mut tasks =
            TaskStore::new(TaskRepository::open_at(path.clone(), false, f3_date(9)).unwrap());
        let mut panel = PanelState::new(f3_date(9));
        let source = tasks.source_revision();
        let job = begin_panel_completion(&mut panel, &mut tasks, source, 0, f3_date(9)).unwrap();
        assert!(tasks.snapshot.tasks[0].completed);
        // Quand le worker découvre un conflit avec le fichier extérieur.
        std::fs::write(&path, "- [ ] Extérieure\n").unwrap();
        let result = job.run();
        assert!(result.error.is_some());
        tasks.finish(&result);
        // Alors l'état extérieur confirmé remplace la complétion ; un brouillon interdit toute nouvelle complétion.
        assert_eq!(tasks.snapshot.tasks[0].title, "Extérieure");
        assert!(!tasks.snapshot.tasks[0].completed);
        panel.begin_edit(&tasks.snapshot, 0);
        panel.editor.as_mut().unwrap().draft = "Brouillon".into();
        let source = tasks.source_revision();
        assert!(begin_panel_completion(&mut panel, &mut tasks, source, 0, f3_date(9)).is_none());
        assert_eq!(panel.editor.as_ref().unwrap().draft, "Brouillon");
        assert!(!tasks.is_busy());
        assert_eq!(std::fs::read_to_string(path).unwrap(), "- [ ] Extérieure\n");
    }

    #[test]
    fn une_edition_optimiste_refusee_restitue_son_brouillon_et_signale_le_conflit() {
        // Architecture §9.1/9.3 — Étant donné une édition affichée avant sa sauvegarde.
        for conflict in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("tasks.md");
            std::fs::write(&path, "- [ ] Originale\n").unwrap();
            let mut tasks =
                TaskStore::new(TaskRepository::open_at(path.clone(), false, f3_date(9)).unwrap());
            let mut panel = PanelState::new(f3_date(9));
            panel.begin_edit(&tasks.snapshot, 0);
            let editor = panel.editor.as_mut().unwrap();
            editor.draft = "  Nouveau titre #mission /p 2026-10-12  ".into();
            editor.help = true;
            let input = parse_input(&editor.draft, f3_date(9)).unwrap();
            let job = tasks
                .begin_write(
                    tasks.source_revision(),
                    TaskCommand::Update { position: 0, input },
                )
                .unwrap();
            let editor = panel.editor.take().unwrap();
            let draft = editor.draft.clone();
            assert_eq!(tasks.snapshot.tasks[0].title, "Nouveau titre");
            // Quand un conflit extérieur ou une erreur de stockage empêche le commit.
            if conflict {
                std::fs::write(&path, "- [ ] Extérieure\n").unwrap();
            } else {
                std::fs::remove_file(&path).unwrap();
                std::fs::create_dir(&path).unwrap();
            }
            let result = job.run();
            assert!(result.error.is_some());
            assert!(tasks.finish(&result));
            assert!(finish_edit(&mut panel, editor, &result));
            panel.refresh(&tasks.snapshot);
            // Alors le brouillon exact revient avec son aide et une erreur visible.
            let editor = panel.editor.as_ref().unwrap();
            assert_eq!(editor.draft, draft);
            assert!(editor.help);
            assert!(editor.error.is_some());
            assert_eq!(editor.invalidated, conflict);
            assert_eq!(
                tasks.snapshot.tasks[0].title,
                if conflict { "Extérieure" } else { "Originale" }
            );
            if conflict {
                assert_eq!(
                    std::fs::read_to_string(&path).unwrap(),
                    "- [ ] Extérieure\n"
                );
            }
        }
    }

    fn press(named: keyboard::key::Named) -> iced::Event {
        let key = keyboard::Key::Named(named);
        iced::Event::Keyboard(keyboard::Event::KeyPressed {
            key: key.clone(),
            modified_key: key,
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::empty(),
            text: None,
            repeat: false,
        })
    }

    fn operate_in_test(
        element: &mut Element<'_, Message>,
        tree: &mut widget::Tree,
        layout: iced::advanced::Layout<'_>,
        renderer: &iced::Renderer,
        mut operation: Box<dyn widget::Operation<Message>>,
    ) -> Option<Message> {
        loop {
            element.as_widget_mut().operate(
                tree,
                layout,
                renderer,
                &mut operation::black_box(operation.as_mut()),
            );
            match operation.finish() {
                Outcome::Chain(next) => operation = next,
                Outcome::Some(message) => return Some(message),
                Outcome::None => return None,
            }
        }
    }

    #[test]
    fn tab_puis_entree_active_un_tag_et_permet_de_retirer_le_filtre() {
        // RG-F2-12/14 — Étant donné de vrais widgets Iced et une capture ouverte à côté du panneau.
        let id = window::Id::unique();
        let mut element: Element<'_, Message> = iced::widget::column![
            text_input("Capture", "")
                .id("capture-title")
                .on_input(move |draft| Message::CaptureChanged(id, draft)),
            keyboard_button(
                "panel-tag-0-0",
                "#Mission".into(),
                Message::FilterTag(id, Some("Mission".into()))
            ),
            keyboard_button(
                "panel-clear-tag",
                "Retirer le filtre".into(),
                Message::FilterTag(id, None)
            ),
        ]
        .into();
        let renderer = iced::Renderer::new(iced::Font::DEFAULT, iced::Pixels(16.0));
        let mut tree = widget::Tree::new(&element);
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &iced::advanced::layout::Limits::new(iced::Size::ZERO, iced::Size::new(430.0, 550.0)),
        );
        let layout = iced::advanced::Layout::new(&node);
        let mut panel = PanelState::new(NaiveDate::from_ymd_opt(2026, 10, 9).unwrap());
        let snapshot = Snapshot {
            revision: 0,
            tasks: mystline::repository::codec::parse(
                "- [ ] Mission #mission\n- [ ] Autre\n",
                panel.today,
            )
            .unwrap(),
        };
        let ids = vec![
            widget::Id::new("panel-tag-0-0"),
            widget::Id::new("panel-clear-tag"),
        ];
        // Quand Tab focalise le tag puis Entrée l'active, et Tab/Entrée active Retirer.
        for expected in [Some("Mission"), None] {
            operate_in_test(
                &mut element,
                &mut tree,
                layout,
                &renderer,
                Box::new(PanelTab {
                    ids: ids.clone(),
                    focused: None,
                    previous: false,
                }),
            );
            let mut messages = Vec::new();
            let mut shell = iced::advanced::Shell::new(&mut messages);
            element.as_widget_mut().update(
                &mut tree,
                &press(keyboard::key::Named::Enter),
                layout,
                iced::mouse::Cursor::Unavailable,
                &renderer,
                &mut iced::advanced::clipboard::Null,
                &mut shell,
                &Rectangle::with_size(iced::Size::new(430.0, 550.0)),
            );
            assert!(shell.is_event_captured());
            assert_eq!(messages.len(), 1);
            match messages.pop().unwrap() {
                Message::FilterTag(window, tag) => {
                    assert_eq!(window, id);
                    assert_eq!(tag.as_deref(), expected);
                    panel.tag = tag;
                }
                other => panic!("Activation attendue du filtre, reçu : {other:?}"),
            }
            // Alors le filtre s'applique et se retire ; la capture n'a pas reçu le focus.
            assert_eq!(
                panel.indices(&snapshot).len(),
                if expected.is_some() { 1 } else { 2 }
            );
        }
    }

    #[test]
    fn une_touche_dedition_capturee_ne_declenche_pas_ledition_du_panneau() {
        // RG-F2-14 — Étant donné Entrée déjà traitée par un champ ou un bouton.
        let id = window::Id::unique();
        // Quand l'abonnement clavier reçoit la même touche.
        assert!(
            keyboard_message(
                keyboard::Key::Named(keyboard::key::Named::Enter),
                keyboard::Modifiers::empty(),
                event::Status::Captured,
                id
            )
            .is_none()
        );
        // Alors il n'ouvre pas une seconde interaction ; Échap et F1 restent disponibles même dans un champ.
        for (key, action) in [
            (keyboard::key::Named::Escape, PanelKey::Escape),
            (keyboard::key::Named::F1, PanelKey::Help),
        ] {
            assert!(
                matches!(keyboard_message(keyboard::Key::Named(key), keyboard::Modifiers::empty(), event::Status::Captured, id), Some(Message::Key(window, received)) if window == id && received == action)
            );
        }
    }

    #[test]
    fn les_fleches_dans_un_champ_texte_ne_deplacent_pas_la_selection() {
        // RG-F2-14 — Étant donné le champ réel de recherche focalisé dans Iced.
        let id = window::Id::unique();
        let mut element: Element<'_, Message> = text_input("Recherche", "support")
            .id("panel-search")
            .on_input(move |query| Message::SearchChanged(id, query))
            .into();
        let renderer = iced::Renderer::new(iced::Font::DEFAULT, iced::Pixels(16.0));
        let mut tree = widget::Tree::new(&element);
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &iced::advanced::layout::Limits::new(iced::Size::ZERO, iced::Size::new(430.0, 550.0)),
        );
        let layout = iced::advanced::Layout::new(&node);
        operate_in_test(
            &mut element,
            &mut tree,
            layout,
            &renderer,
            Box::new(operation::focusable::focus(widget::Id::new("panel-search"))),
        );
        // Quand une commande de navigation vérifie le focus courant.
        let result = operate_in_test(
            &mut element,
            &mut tree,
            layout,
            &renderer,
            Box::new(TextFocus {
                id,
                key: PanelKey::Move(true),
                focused: false,
            }),
        );
        // Alors elle est marquée comme provenant d'un champ, pour que la navigation du panneau l'ignore.
        assert!(
            matches!(result, Some(Message::PanelKey(window, PanelKey::Move(true), true)) if window == id)
        );
    }

    #[test]
    fn une_capture_invalide_conserve_sa_fenetre_son_texte_et_le_fichier() {
        // RG-F2-04 — Étant donné les saisies refusées par le véritable démarrage de capture.
        for draft in [
            "Support /d",
            "Support /p 2026-02-29",
            "Support /p lundi /p mardi",
            "/p lundi #mission",
        ] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("tasks.md");
            let mut tasks = TaskStore::new(TaskRepository::open(path.clone(), true).unwrap());
            let mut capture = QuickCapture::default();
            capture.edit(draft.into());
            let id = window::Id::unique();
            // Quand Entrée soumet la saisie.
            assert!(
                start_capture(
                    Message::CaptureSubmitted(id),
                    Some(id),
                    &mut capture,
                    &mut tasks
                )
                .is_none()
            );
            // Alors aucune opération n'est lancée et le texte reste récupérable.
            assert_eq!(capture.draft, draft);
            assert!(capture.error.is_some());
            assert_eq!(std::fs::read_to_string(path).unwrap(), "");
            assert_eq!(tasks.snapshot.revision, 0);
            assert!(!tasks.is_busy());
        }
    }

    #[test]
    fn les_actions_visent_la_selection_visible_apres_filtrage_et_permutation() {
        // RG-F2-13/14 — Étant donné une ligne masquée et deux tâches visibles.
        let today = NaiveDate::from_ymd_opt(2026, 10, 9).unwrap();
        let mut snapshot = Snapshot {
            revision: 0,
            tasks: mystline::repository::codec::parse(
                "- [ ] Masquée\n- [ ] A #visible\n- [ ] B #visible\n",
                today,
            )
            .unwrap(),
        };
        let mut panel = PanelState::new(today);
        let mut navigation = PanelNavigation::default();
        panel.tag = Some("visible".into());
        let indices = panel.indices(&snapshot);
        navigation.open(indices.len());
        navigation.next(indices.len());
        // Quand la sélection est parcourue puis B déplacée à la place de A.
        let from = navigation.selected_index(&indices).unwrap();
        assert_eq!(snapshot.tasks[from].title, "B");
        let to = panel.neighbor(&snapshot, from, false).unwrap();
        snapshot.tasks.swap(from, to);
        let indices = panel.indices(&snapshot);
        navigation.selected = indices.iter().position(|&index| index == to);
        // Alors la sélection suit B et l'édition concerne toujours B.
        panel.begin_edit(&snapshot, navigation.selected_index(&indices).unwrap());
        assert_eq!(panel.editor.as_ref().unwrap().draft, "B #visible");
        panel.query = "introuvable".into();
        navigation.refresh(panel.indices(&snapshot).len());
        assert_eq!(navigation.selected, None);
    }

    #[test]
    fn effacer_la_recherche_conserve_la_selection_avant_edition() {
        // RG-F2-11/14 — Étant donné Other, Support A et Support B, avec la recherche « support ».
        let today = NaiveDate::from_ymd_opt(2026, 10, 9).unwrap();
        let snapshot = Snapshot {
            revision: 0,
            tasks: mystline::repository::codec::parse(
                "- [ ] Other\n- [ ] Support A\n- [ ] Support B\n",
                today,
            )
            .unwrap(),
        };
        let mut panel = PanelState::new(today);
        panel.search_active = true;
        panel.query = "support".into();
        let mut navigation = PanelNavigation::default();
        navigation.open(panel.indices(&snapshot).len());
        navigation.next(panel.indices(&snapshot).len());
        let filtered_selection = navigation
            .selected_index(&panel.indices(&snapshot))
            .unwrap();
        assert_eq!(snapshot.tasks[filtered_selection].title, "Support B");

        // Quand Échap efface la recherche puis Entrée ouvre l'édition.
        assert!(escape_panel_view(&mut panel, &mut navigation, &snapshot));
        let selected = navigation
            .selected_index(&panel.indices(&snapshot))
            .unwrap();
        panel.begin_edit(&snapshot, selected);

        // Alors la sélection reste sur Support B et l'édition cible ce titre.
        assert_eq!(snapshot.tasks[selected].title, "Support B");
        assert_eq!(panel.editor.as_ref().unwrap().position, selected);
        assert!(
            panel
                .editor
                .as_ref()
                .unwrap()
                .draft
                .starts_with("Support B")
        );
    }

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
    fn panel_shows_existing_tasks_and_refreshes_after_external_edit_and_source_change() {
        let panel = PanelState::new(f3_date(9));
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.md");
        std::fs::write(
            &path,
            "- [ ] Première\n- [x] Terminée @completed(2026-10-01)\n- [ ] Deuxième\n",
        )
        .unwrap();
        let mut repo = TaskRepository::open_at(path.clone(), false, f3_date(9)).unwrap();
        let mut snapshot = Snapshot {
            revision: 0,
            tasks: vec![],
        };
        let mut navigation = PanelNavigation::default();

        replace_panel_snapshot(&mut snapshot, &mut navigation, &panel, repo.snapshot());
        let active = panel.indices(&snapshot);
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

        std::fs::write(
            &path,
            "- [x] Première @completed(2026-10-01)\n- [ ] Extérieure\n",
        )
        .unwrap();
        assert!(repo.reload_at(f3_date(9)).unwrap());
        replace_panel_snapshot(&mut snapshot, &mut navigation, &panel, repo.snapshot());
        let active = panel.indices(&snapshot);
        assert_eq!(
            active
                .iter()
                .map(|&i| snapshot.tasks[i].title.as_str())
                .collect::<Vec<_>>(),
            ["Extérieure"]
        );
        assert_eq!(navigation.selected_index(&active), Some(1));

        let other = dir.path().join("autre.md");
        std::fs::write(
            &other,
            "- [ ] Autre fichier\n- [x] Ancienne @completed(2026-10-01)\n",
        )
        .unwrap();
        let candidate = TaskRepository::open_at(other, false, f3_date(9)).unwrap();
        replace_panel_snapshot(&mut snapshot, &mut navigation, &panel, candidate.snapshot());
        let active = panel.indices(&snapshot);
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
        // Architecture §9.3 / F1 — Étant donné une capture et une modification extérieure non rechargée.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.md");
        let mut tasks = TaskStore::new(TaskRepository::open(path.clone(), true).unwrap());
        let mut capture = QuickCapture::default();
        let id = window::Id::unique();
        capture.edit("  Première tâche  ".into());
        std::fs::write(&path, "- [ ] Tâche extérieure\n").unwrap();
        // Quand la capture s'affiche optimistement puis son commit découvre le conflit.
        let job = start_capture(
            Message::CaptureSubmitted(id),
            Some(id),
            &mut capture,
            &mut tasks,
        )
        .unwrap();
        assert_eq!(tasks.snapshot.tasks[0].title, "Première tâche");
        let result = job.run();
        tasks.finish(&result);
        assert!(!finish_capture(
            &mut capture,
            Some(id),
            id,
            "  Première tâche  ",
            &result
        ));
        // Alors la capture reste ouverte et intacte ; réessayer confirme ensuite l'écriture.
        assert_eq!(capture.draft, "  Première tâche  ");
        assert!(capture.error.is_some());
        assert_eq!(tasks.snapshot.tasks[0].title, "Tâche extérieure");
        let job = start_capture(
            Message::CaptureSubmitted(id),
            Some(id),
            &mut capture,
            &mut tasks,
        )
        .unwrap();
        let result = job.run();
        tasks.finish(&result);
        assert!(finish_capture(
            &mut capture,
            Some(id),
            id,
            "  Première tâche  ",
            &result
        ));
        assert_eq!(tasks.snapshot.tasks[1].title, "Première tâche");
        assert!(capture.error.is_none());
        assert!(
            std::fs::read_to_string(path)
                .unwrap()
                .contains("- [ ] Première tâche")
        );
    }

    #[test]
    fn capture_submitted_message_keeps_window_and_draft_on_error_then_closes_on_retry() {
        // F1 — Étant donné une capture ouverte et des messages de fenêtres anciennes.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.md");
        let mut tasks = TaskStore::new(TaskRepository::open(path, true).unwrap());
        let mut capture = QuickCapture::default();
        let id = window::Id::unique();
        assert!(
            start_capture(
                Message::CaptureSubmitted(id),
                Some(id),
                &mut capture,
                &mut tasks
            )
            .is_none()
        );
        assert!(capture.error.is_some());
        // Quand une fenêtre courante saisit un titre puis un ancien message essaie de valider.
        start_capture(
            Message::CaptureChanged(id, "Une tâche".into()),
            Some(id),
            &mut capture,
            &mut tasks,
        );
        assert!(
            start_capture(
                Message::CaptureSubmitted(window::Id::unique()),
                Some(id),
                &mut capture,
                &mut tasks
            )
            .is_none()
        );
        assert!(tasks.snapshot.tasks.is_empty());
        let job = start_capture(
            Message::CaptureSubmitted(id),
            Some(id),
            &mut capture,
            &mut tasks,
        )
        .unwrap();
        let result = job.run();
        tasks.finish(&result);
        // Alors seule la bonne fenêtre peut être fermée par la confirmation.
        assert!(!finish_capture(
            &mut capture,
            Some(window::Id::unique()),
            id,
            "Une tâche",
            &result
        ));
        assert!(finish_capture(
            &mut capture,
            Some(id),
            id,
            "Une tâche",
            &result
        ));
        assert_eq!(tasks.snapshot.tasks.len(), 1);
        assert!(
            start_capture(
                Message::CaptureSubmitted(id),
                None,
                &mut capture,
                &mut tasks
            )
            .is_none()
        );
        assert_eq!(tasks.snapshot.tasks.len(), 1);
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
