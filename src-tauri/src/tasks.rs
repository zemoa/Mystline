//! Contrat de présentation : Rust calcule les vues et valide les intentions.
use chrono::NaiveDate;
use mystline::{
    application::TaskCommand,
    presentation::{
        input::parse_edit_input,
        panel::{DeadlineTone, PanelMode, PanelState, SortRank, task_deadline_tone},
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
    pub busy: bool,
    pub rows: Vec<TaskRow>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRow {
    pub repeat: Option<String>,
    pub can_reopen: bool,
    pub reopen_warning: Option<&'static str>,
    pub position: usize,
    pub rank: &'static str,
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
                repeat: task.recurrence.as_ref().map(|r| r.rule.to_string()),
                can_reopen: can_reopen(snapshot, row.index),
                reopen_warning: (task.completed && task.occurrence.is_some() && can_reopen(snapshot, row.index)).then_some("Réouvrir cette occurrence supprimera la suivante et ses éventuelles modifications."),
                position: row.index,
                rank: match row.rank {
                    SortRank::Overdue => "overdue",
                    SortRank::Today => "today",
                    SortRank::Tomorrow => "tomorrow",
                    SortRank::Undated => "undated",
                    SortRank::Later => "later",
                },
                title: task.title.clone(),
                tags: task.tags.clone(),
                planned: task.planned.map(|date| date.to_string()),
                deadline: task.deadline.map(|date| date.to_string()),
                deadline_tone: task_deadline_tone(task, today).map(|tone| match tone {
                    DeadlineTone::Overdue => "overdue",
                    DeadlineTone::Today => "today",
                    DeadlineTone::Tomorrow => "tomorrow",
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
        busy,
        rows,
    }
}

fn can_reopen(snapshot: &Snapshot, position: usize) -> bool {
    let task = &snapshot.tasks[position];
    !task.occurrence.is_some_and(|id| {
        snapshot.tasks.iter().any(|other| {
            other.completed
                && other
                    .occurrence
                    .is_some_and(|next| next.series == id.series && next.generation > id.generation)
        })
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepeatPreview {
    pub needs_date: bool,
    pub draft: String,
    pub repeat: Option<String>,
    pub planned: Option<String>,
    pub deadline: Option<String>,
    pub dates: Vec<PreviewDate>,
    pub requires_confirmation: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewDate {
    pub planned: String,
    pub deadline: Option<String>,
}

pub fn preview(
    snapshot: &Snapshot,
    position: Option<usize>,
    draft: &str,
    today: NaiveDate,
) -> Result<RepeatPreview, String> {
    let mut input = if position.is_some() {
        parse_edit_input(draft, today)
    } else {
        mystline::presentation::input::parse_input(draft, today)
    }
    .map_err(|e| e.to_string())?;
    let missing_date = input.repeat.is_some() && input.planned.is_none();
    if missing_date {
        input.planned = input.repeat.as_ref().and_then(|r| r.first(today));
    }
    let requires_confirmation = if let Some(position) = position {
        let old = snapshot
            .tasks
            .get(position)
            .ok_or("La tâche n'existe plus.")?;
        missing_date
            || old.recurrence.as_ref().map(|r| &r.rule) != input.repeat.as_ref()
            || (input.repeat.is_some() && input.deadline != old.deadline)
    } else {
        false
    };
    let command = position.map_or_else(
        || TaskCommand::Create(input.clone()),
        |position| TaskCommand::Update {
            position,
            input: input.clone(),
        },
    );
    let proposed =
        mystline::application::propose_task_change(snapshot, snapshot.revision, &command)
            .map_err(|e| e.to_string())?;
    let task = proposed
        .tasks
        .get(position.unwrap_or(snapshot.tasks.len()))
        .ok_or("Aucune tâche")?;
    let mut dates = Vec::new();
    if let Some(r) = &task.recurrence {
        let mut reference = r.reference;
        for _ in 0..3 {
            dates.push(PreviewDate {
                planned: reference.to_string(),
                deadline: r
                    .deadline_offset
                    .and_then(|n| {
                        chrono::Duration::try_days(n).and_then(|d| reference.checked_add_signed(d))
                    })
                    .map(|d| d.to_string()),
            });
            reference = r
                .rule
                .after(reference, r.anchor)
                .ok_or("Le calendrier dépasse les dates prises en charge.")?;
        }
    }
    Ok(RepeatPreview {
        needs_date: missing_date,
        draft: mystline::presentation::input::format_edit(task),
        repeat: task.recurrence.as_ref().map(|r| r.rule.to_string()),
        planned: task.planned.map(|d| d.to_string()),
        deadline: task.deadline.map(|d| d.to_string()),
        dates,
        requires_confirmation,
    })
}

pub fn update(position: usize, draft: &str, today: NaiveDate) -> Result<TaskCommand, String> {
    Ok(TaskCommand::Update {
        position,
        input: parse_edit_input(draft, today).map_err(|error| error.to_string())?,
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
        if !can_reopen(snapshot, position) {
            return Err("La suivante est déjà terminée : cette ancienne occurrence ne peut plus être rouverte.".into());
        }
        TaskCommand::Reopen { position }
    } else {
        TaskCommand::Complete { position, today }
    })
}

pub fn delete(
    snapshot: &Snapshot,
    request: &ViewRequest,
    position: usize,
    today: NaiveDate,
) -> Result<TaskCommand, String> {
    if request.history || !panel(request, today).indices(snapshot).contains(&position) {
        return Err("Cette tâche n'est pas modifiable dans cette vue.".into());
    }
    Ok(TaskCommand::Delete { position })
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
