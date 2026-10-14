//! Contrat de présentation : Rust calcule les vues et valide les intentions.
use chrono::NaiveDate;
use mystline::{
    application::TaskCommand,
    presentation::{
        input::parse_input,
        panel::{DeadlineTone, PanelMode, PanelState, Section, next_workday, task_deadline_tone},
    },
    repository::Snapshot,
    runtime::task_store::SourceRevision,
};
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewRequest {
    #[serde(default)]
    pub query: String,
    pub tag: Option<String>,
    #[serde(default)]
    pub history: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskView {
    pub version: u64,
    pub source: SourceRevision,
    pub today: String,
    pub next_workday: String,
    pub busy: bool,
    pub rows: Vec<TaskRow>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRow {
    pub position: usize,
    pub section: &'static str,
    pub title: String,
    pub tags: Vec<String>,
    pub planned: Option<String>,
    pub deadline: Option<String>,
    pub deadline_tone: Option<&'static str>,
    pub completed: bool,
    pub completed_date: Option<String>,
    pub edit_draft: String,
}

pub fn panel(request: &ViewRequest, today: NaiveDate) -> PanelState {
    let mut panel = PanelState::new(today);
    panel.query.clone_from(&request.query);
    panel.tag.clone_from(&request.tag);
    panel.mode = if request.history {
        PanelMode::History
    } else {
        PanelMode::Daily
    };
    panel
}

pub fn project(
    snapshot: &Snapshot,
    source: SourceRevision,
    busy: bool,
    request: &ViewRequest,
    today: NaiveDate,
    version: u64,
) -> TaskView {
    let rows = panel(request, today)
        .rows(snapshot)
        .into_iter()
        .map(|row| {
            let task = &snapshot.tasks[row.index];
            TaskRow {
                position: row.index,
                section: match row.section {
                    Section::Overdue => "overdue",
                    Section::Today => "today",
                    Section::NextWorkday => "nextWorkday",
                    Section::All => "all",
                },
                title: task.title.clone(),
                tags: task.tags.clone(),
                planned: task.planned.map(|date| date.to_string()),
                deadline: task.deadline.map(|date| date.to_string()),
                deadline_tone: task_deadline_tone(task, today).map(|tone| match tone {
                    DeadlineTone::Overdue => "overdue",
                    DeadlineTone::Today => "today",
                    DeadlineTone::Future => "future",
                }),
                completed: task.completed,
                completed_date: task
                    .effective_completion_date()
                    .map(|date| date.to_string()),
                edit_draft: mystline::presentation::input::format_edit(task),
            }
        })
        .collect();
    TaskView {
        version,
        source,
        today: today.to_string(),
        next_workday: next_workday(today).to_string(),
        busy,
        rows,
    }
}

pub fn update(position: usize, draft: &str, today: NaiveDate) -> Result<TaskCommand, String> {
    Ok(TaskCommand::Update {
        position,
        input: parse_input(draft, today).map_err(|error| error.to_string())?,
    })
}

pub fn toggle(
    snapshot: &Snapshot,
    request: &ViewRequest,
    position: usize,
    today: NaiveDate,
) -> Result<TaskCommand, String> {
    if request.history || !panel(request, today).indices(snapshot).contains(&position) {
        return Err("Cette tâche n'est pas modifiable dans cette vue.".into());
    }
    let task = snapshot
        .tasks
        .get(position)
        .ok_or("La tâche sélectionnée n'existe plus.")?;
    Ok(if task.completed {
        TaskCommand::Reopen { position }
    } else {
        TaskCommand::Complete { position, today }
    })
}

pub fn reorder(
    snapshot: &Snapshot,
    request: &ViewRequest,
    position: usize,
    down: bool,
    today: NaiveDate,
) -> Option<TaskCommand> {
    panel(request, today)
        .neighbor(snapshot, position, down)
        .map(|to| TaskCommand::Reorder { from: position, to })
}

#[cfg(test)]
#[path = "tasks_tests.rs"]
mod tests;
