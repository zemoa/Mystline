pub mod codec;

use crate::domain::Task;
use chrono::{Local, NaiveDate};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub revision: u64,
    pub tasks: Vec<Task>,
}

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("{0}")]
    Codec(#[from] codec::CodecError),
    #[error("le fichier a été modifié depuis la dernière lecture")]
    Changed,
    #[error("révision obsolète")]
    Stale,
}

pub struct TaskRepository {
    path: PathBuf,
    fingerprint: blake3::Hash,
    snapshot: Snapshot,
}

impl TaskRepository {
    pub fn open(path: PathBuf, create_missing: bool) -> Result<Self, RepositoryError> {
        Self::open_at(path, create_missing, Local::now().date_naive())
    }

    /// La date explicite permet de simuler une observation ou un redémarrage.
    pub fn open_at(
        path: PathBuf,
        create_missing: bool,
        today: NaiveDate,
    ) -> Result<Self, RepositoryError> {
        if create_missing && !path.exists() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            // create_new avoids clobbering an external editor's file.
            match File::create_new(&path) {
                Ok(_) => (),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => (),
                Err(e) => return Err(e.into()),
            }
        }
        let bytes = fs::read(&path)?;
        let text = std::str::from_utf8(&bytes)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let tasks = codec::parse(text, today)?;
        Ok(Self {
            path,
            fingerprint: blake3::hash(&bytes),
            snapshot: Snapshot { revision: 0, tasks },
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn snapshot(&self) -> Snapshot {
        self.snapshot.clone()
    }

    /// Invalid external edits never replace the last confirmed state or touch the file.
    pub fn reload(&mut self) -> Result<bool, RepositoryError> {
        self.reload_at(Local::now().date_naive())
    }

    pub fn reload_at(&mut self, today: NaiveDate) -> Result<bool, RepositoryError> {
        let bytes = fs::read(&self.path)?;
        let hash = blake3::hash(&bytes);
        if hash == self.fingerprint {
            return Ok(false);
        }
        let text = std::str::from_utf8(&bytes)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let mut tasks = codec::parse(text, today)?;
        // Physical positions can change externally. Only an unambiguous,
        // unchanged task carries its observation date across reloads.
        let old_observations = observation_index(&self.snapshot.tasks);
        let new_observations = observation_index(&tasks);
        let observations: Vec<_> = tasks
            .iter()
            .map(|task| {
                let key = UndatedTask::from(task)?;
                let &(new_count, _) = new_observations.get(&key)?;
                let &(old_count, observed) = old_observations.get(&key)?;
                (new_count == 1 && old_count == 1)
                    .then_some(observed)
                    .flatten()
            })
            .collect();
        for (task, observed) in tasks.iter_mut().zip(observations) {
            if observed.is_some() {
                task.observed_completion = observed;
            }
        }
        self.fingerprint = hash;
        self.snapshot.tasks = tasks;
        self.snapshot.revision += 1;
        Ok(true)
    }

    pub fn commit(
        &mut self,
        revision: u64,
        mut tasks: Vec<Task>,
    ) -> Result<Snapshot, RepositoryError> {
        if revision != self.snapshot.revision {
            return Err(RepositoryError::Stale);
        }
        if blake3::hash(&fs::read(&self.path)?) != self.fingerprint {
            self.reload()?;
            return Err(RepositoryError::Changed);
        }
        for task in &mut tasks {
            if task.completed {
                task.completed_date = task.effective_completion_date();
                task.observed_completion = None;
            }
        }
        let content = codec::serialize(&tasks);
        let parent = self.path.parent().unwrap_or_else(|| Path::new("."));
        let mut temp = tempfile::NamedTempFile::new_in(parent)?;
        temp.write_all(content.as_bytes())?;
        temp.as_file().sync_all()?;
        replace(temp, &self.path)?;
        self.fingerprint = blake3::hash(content.as_bytes());
        self.snapshot.tasks = tasks;
        self.snapshot.revision += 1;
        Ok(self.snapshot())
    }
}

#[derive(PartialEq, Eq, Hash)]
struct UndatedTask<'a> {
    title: &'a str,
    planned: Option<NaiveDate>,
    deadline: Option<NaiveDate>,
    tags: &'a [String],
}

impl<'a> UndatedTask<'a> {
    fn from(task: &'a Task) -> Option<Self> {
        (task.completed && task.completed_date.is_none()).then_some(Self {
            title: &task.title,
            planned: task.planned,
            deadline: task.deadline,
            tags: &task.tags,
        })
    }
}

fn observation_index(tasks: &[Task]) -> HashMap<UndatedTask<'_>, (usize, Option<NaiveDate>)> {
    let mut index = HashMap::new();
    for task in tasks {
        if let Some(key) = UndatedTask::from(task) {
            let entry = index.entry(key).or_insert((0, task.observed_completion));
            entry.0 += 1;
        }
    }
    index
}

fn replace(file: tempfile::NamedTempFile, path: &Path) -> io::Result<()> {
    file.persist(path).map_err(|e| e.error)?;
    // Once renamed, the commit is visible; a directory sync failure cannot roll it back.
    #[cfg(unix)]
    if let Some(parent) = path.parent() {
        let _ = File::open(parent).and_then(|dir| dir.sync_all());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn un_grand_fichier_reordonne_conserve_les_dates_sans_recriture() {
        // Architecture §2.9/12 — Étant donné 10 000 lignes cochées sans date.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.md");
        let first_day = NaiveDate::from_ymd_opt(2026, 10, 9).unwrap();
        let next_day = first_day.succ_opt().unwrap();
        let original: String = (0..10_000)
            .map(|n| format!("- [x] Tâche {n} #mission\n"))
            .collect();
        fs::write(&path, original).unwrap();
        let mut repo = TaskRepository::open_at(path.clone(), false, first_day).unwrap();
        // Quand l'éditeur inverse toute la liste et ajoute une tâche active le lendemain.
        let external: String = std::iter::once("- [ ] Nouvelle\n".into())
            .chain(
                (0..10_000)
                    .rev()
                    .map(|n| format!("- [x] Tâche {n} #mission\n")),
            )
            .collect();
        fs::write(&path, &external).unwrap();
        assert!(repo.reload_at(next_day).unwrap());
        // Alors toutes les anciennes dates restent conservées et le fichier demeure extérieur.
        let snapshot = repo.snapshot();
        assert_eq!(snapshot.tasks.len(), 10_001);
        assert!(!snapshot.tasks[0].completed);
        assert_eq!(snapshot.tasks[1].title, "Tâche 9999");
        assert!(
            snapshot.tasks[1..]
                .iter()
                .all(|task| task.effective_completion_date() == Some(first_day))
        );
        assert_eq!(fs::read_to_string(path).unwrap(), external);
    }

    #[test]
    fn external_invalid_content_and_lost_update() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.md");
        let mut repo = TaskRepository::open(path.clone(), true).unwrap();
        fs::write(&path, "en cours de sauvegarde").unwrap();
        assert!(repo.reload().is_err());
        assert_eq!(repo.snapshot().revision, 0);
        fs::write(&path, "- [ ] Nouvelle tâche\n").unwrap();
        assert!(matches!(
            repo.commit(0, vec![]),
            Err(RepositoryError::Changed)
        ));
        assert_eq!(repo.snapshot().tasks[0].title, "Nouvelle tâche");
        assert_eq!(fs::read_to_string(&path).unwrap(), "- [ ] Nouvelle tâche\n");
    }

    #[test]
    fn confirmed_commit_is_atomic_and_revisioned() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.md");
        let mut repo = TaskRepository::open(path.clone(), true).unwrap();
        let tasks =
            codec::parse("- [ ] Nouvelle tâche #travail\n", Local::now().date_naive()).unwrap();
        let snapshot = repo.commit(0, tasks.clone()).unwrap();
        assert_eq!(snapshot.revision, 1);
        assert_eq!(snapshot.tasks, tasks);
        assert!(matches!(
            repo.commit(0, vec![]),
            Err(RepositoryError::Stale)
        ));
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "- [ ] Nouvelle tâche #travail\n"
        );
        assert!(!repo.reload().unwrap()); // no duplicate watcher update after our own write
    }

    #[test]
    fn external_completion_is_not_rewritten_by_reload() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.md");
        let mut repo = TaskRepository::open(path.clone(), true).unwrap();
        fs::write(&path, "- [x] Vérifier sans date\n").unwrap();
        assert!(repo.reload().unwrap());
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "- [x] Vérifier sans date\n"
        );
        let snapshot = repo.snapshot();
        assert!(snapshot.tasks[0].effective_completion_date().is_some());
        repo.commit(snapshot.revision, snapshot.tasks).unwrap();
        assert!(fs::read_to_string(&path).unwrap().contains("@completed("));
    }
}
