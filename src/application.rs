use crate::{
    domain::{Task, same_tag, valid_tag},
    repository::{RepositoryError, Snapshot, TaskRepository},
};
use chrono::NaiveDate;
use thiserror::Error;

/// Données métier uniquement : la syntaxe des commandes appartient à la présentation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskInput {
    pub title: String,
    pub planned: Option<NaiveDate>,
    pub deadline: Option<NaiveDate>,
    pub tags: Vec<String>,
}

#[derive(Debug, Error)]
pub enum CreateTaskError {
    #[error("le titre de la tâche est vide")]
    EmptyTitle,
    #[error("tag invalide : {0}")]
    InvalidTag(String),
    #[error("la tâche sélectionnée n'est plus modifiable")]
    InvalidPosition,
    #[error("impossible d'enregistrer la tâche : {0}")]
    Repository(#[from] RepositoryError),
}

pub fn create_task(repo: &mut TaskRepository, title: &str) -> Result<Snapshot, CreateTaskError> {
    create_task_from_input(
        repo,
        &TaskInput {
            title: title.into(),
            ..TaskInput::default()
        },
    )
}

fn validate(input: &TaskInput) -> Result<TaskInput, CreateTaskError> {
    let mut input = input.clone();
    input.title = input.title.trim().to_owned();
    if input.title.is_empty() {
        return Err(CreateTaskError::EmptyTitle);
    }
    let mut tags: Vec<String> = Vec::new();
    for tag in input.tags {
        if !valid_tag(&tag) {
            return Err(CreateTaskError::InvalidTag(tag));
        }
        if !tags.iter().any(|old| same_tag(old, &tag)) {
            tags.push(tag);
        }
    }
    input.tags = tags;
    Ok(input)
}

pub fn create_task_from_input(
    repo: &mut TaskRepository,
    input: &TaskInput,
) -> Result<Snapshot, CreateTaskError> {
    let input = validate(input)?;
    let mut snapshot = repo.snapshot();
    snapshot.tasks.push(Task {
        title: input.title,
        planned: input.planned,
        deadline: input.deadline,
        tags: input.tags,
        completed: false,
        completed_date: None,
        observed_completion: None,
    });
    Ok(repo.commit(snapshot.revision, snapshot.tasks)?)
}

pub fn update_task(
    repo: &mut TaskRepository,
    revision: u64,
    position: usize,
    input: &TaskInput,
) -> Result<Snapshot, CreateTaskError> {
    let input = validate(input)?;
    let mut snapshot = repo.snapshot();
    if snapshot.revision != revision {
        return Err(RepositoryError::Stale.into());
    }
    let task = snapshot
        .tasks
        .get_mut(position)
        .filter(|task| !task.completed)
        .ok_or(CreateTaskError::InvalidPosition)?;
    task.title = input.title;
    task.planned = input.planned;
    task.deadline = input.deadline;
    task.tags = input.tags;
    Ok(repo.commit(revision, snapshot.tasks)?)
}

/// La présentation fournit deux voisins visibles de la même section.
/// Une permutation laisse toutes les autres lignes à leur position exacte.
pub fn reorder_tasks(
    repo: &mut TaskRepository,
    revision: u64,
    from: usize,
    to: usize,
) -> Result<Snapshot, CreateTaskError> {
    let mut snapshot = repo.snapshot();
    if snapshot.revision != revision {
        return Err(RepositoryError::Stale.into());
    }
    for position in [from, to] {
        if snapshot
            .tasks
            .get(position)
            .is_none_or(|task| task.completed)
        {
            return Err(CreateTaskError::InvalidPosition);
        }
    }
    if from == to {
        return Ok(snapshot);
    }
    snapshot.tasks.swap(from, to);
    Ok(repo.commit(revision, snapshot.tasks)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn persists_an_active_title_only_task_and_reopens_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.md");
        let mut repo = TaskRepository::open(path.clone(), true).unwrap();

        let snapshot = create_task(&mut repo, "  Préparer  le dossier\tfinal  ").unwrap();
        assert_eq!(snapshot.revision, 1);
        assert_eq!(snapshot.tasks.len(), 1);
        let task = &snapshot.tasks[0];
        assert_eq!(task.title, "Préparer  le dossier\tfinal");
        assert!(!task.completed);
        assert_eq!(task.planned, None);
        assert_eq!(task.deadline, None);
        assert!(task.tags.is_empty());
        assert_eq!(task.completed_date, None);
        assert_eq!(task.observed_completion, None);
        assert_eq!(repo.snapshot().tasks, snapshot.tasks);
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "- [ ] Préparer  le dossier\tfinal\n"
        );

        let reopened = TaskRepository::open(path, false).unwrap().snapshot();
        assert_eq!(reopened.tasks, snapshot.tasks);
    }

    #[test]
    fn rejects_whitespace_only_without_writing_or_changing_cache() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.md");
        let mut repo = TaskRepository::open(path.clone(), true).unwrap();
        let before = repo.snapshot();

        let error = create_task(&mut repo, " \n\t  ").unwrap_err();
        assert!(matches!(error, CreateTaskError::EmptyTitle));
        assert!(!error.to_string().is_empty());
        assert_eq!(repo.snapshot().revision, before.revision);
        assert_eq!(repo.snapshot().tasks, before.tasks);
        assert_eq!(fs::read_to_string(path).unwrap(), "");
    }

    #[test]
    fn literal_hash_and_metadata_like_words_survive_commit_and_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.md");
        let mut repo = TaskRepository::open(path.clone(), true).unwrap();
        let title = r"Relire #architecture @deadline(2026-10-05) \#mission \@planned(2026-10-06)";

        let snapshot = create_task(&mut repo, title).unwrap();
        assert_eq!(snapshot.tasks[0].title, title);
        assert!(snapshot.tasks[0].tags.is_empty());
        assert_eq!(snapshot.tasks[0].deadline, None);
        assert_eq!(snapshot.tasks[0].planned, None);
        let markdown = fs::read_to_string(&path).unwrap();
        assert!(markdown.contains(r"\#architecture"));
        assert!(markdown.contains(r"\@deadline(2026-10-05)"));
        assert_eq!(
            TaskRepository::open(path, false).unwrap().snapshot().tasks,
            snapshot.tasks
        );
    }

    #[test]
    fn capturing_does_not_change_legacy_titles_with_backslashes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.md");
        let original = "Sauver \\\\serveur\\dossier et \\#mission";
        fs::write(&path, format!("- [ ] {original}\n")).unwrap();
        let mut repo = TaskRepository::open(path.clone(), false).unwrap();

        create_task(&mut repo, "Nouvelle tâche").unwrap();

        let reopened = TaskRepository::open(path.clone(), false)
            .unwrap()
            .snapshot();
        assert_eq!(reopened.tasks[0].title, original);
        assert_eq!(reopened.tasks[1].title, "Nouvelle tâche");
        assert!(
            fs::read_to_string(path)
                .unwrap()
                .contains("@title-escaped(1)")
        );
    }

    #[test]
    fn title_prefixes_and_internal_spaces_round_trip_without_reinterpreting_external_tags() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.md");
        fs::write(
            &path,
            "- [ ] Dossier  externe #mission @planned(2026-10-05)\n",
        )
        .unwrap();
        let mut repo = TaskRepository::open(path.clone(), false).unwrap();
        let cases = [
            ("#architecture", r"\#architecture"),
            ("#", r"\#"),
            (r"\#mission", r"\\#mission"),
            ("  Préparer  le\t dossier  ", "Préparer  le\t dossier"),
        ];

        for (input, encoded) in cases {
            let confirmed = create_task(&mut repo, input).unwrap();
            let title = input.trim();
            assert_eq!(confirmed.tasks.last().unwrap().title, title);
            assert!(confirmed.tasks.last().unwrap().tags.is_empty());
            let markdown = fs::read_to_string(&path).unwrap();
            let marker = if encoded == title {
                ""
            } else {
                " @title-escaped(1)"
            };
            assert!(
                markdown
                    .lines()
                    .any(|line| line == format!("- [ ] {encoded}{marker}"))
            );
            assert_eq!(
                crate::repository::codec::parse(&markdown, chrono::Local::now().date_naive())
                    .unwrap(),
                confirmed.tasks
            );
            let reopened = TaskRepository::open(path.clone(), false)
                .unwrap()
                .snapshot();
            assert_eq!(reopened.tasks, confirmed.tasks);
            assert_eq!(reopened.tasks[0].title, "Dossier  externe");
            assert_eq!(reopened.tasks[0].tags, ["mission"]);
            assert_eq!(
                reopened.tasks[0].planned,
                chrono::NaiveDate::from_ymd_opt(2026, 10, 5)
            );
        }
    }

    #[test]
    fn failed_commit_does_not_confirm_or_change_cache() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.md");
        let mut repo = TaskRepository::open(path.clone(), true).unwrap();
        create_task(&mut repo, "Existante").unwrap();
        let before = repo.snapshot();
        fs::remove_file(&path).unwrap();

        let error = create_task(&mut repo, "Non confirmée").unwrap_err();
        assert!(matches!(
            error,
            CreateTaskError::Repository(RepositoryError::Io(_))
        ));
        assert_eq!(repo.snapshot().revision, before.revision);
        assert_eq!(repo.snapshot().tasks, before.tasks);
    }

    #[test]
    fn conflict_reloads_external_edit_and_does_not_confirm_proposed_task() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.md");
        let mut repo = TaskRepository::open(path.clone(), true).unwrap();
        create_task(&mut repo, "Existante").unwrap();
        fs::write(&path, "- [ ] Extérieure #mission\n").unwrap();

        let error = create_task(&mut repo, "Non confirmée").unwrap_err();
        assert!(matches!(
            error,
            CreateTaskError::Repository(RepositoryError::Changed)
        ));
        assert_eq!(repo.snapshot().revision, 2);
        assert_eq!(repo.snapshot().tasks.len(), 1);
        assert_eq!(repo.snapshot().tasks[0].title, "Extérieure");
        assert_eq!(repo.snapshot().tasks[0].tags, ["mission"]);
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "- [ ] Extérieure #mission\n"
        );
        assert_eq!(
            create_task(&mut repo, "Après recharge").unwrap().revision,
            3
        );
        assert_eq!(repo.snapshot().tasks.len(), 2);
    }
}
