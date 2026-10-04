pub mod codec;

use crate::domain::Task;
use chrono::Local;
use std::{
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
        let tasks = codec::parse(text, Local::now().date_naive())?;
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
        let bytes = fs::read(&self.path)?;
        let hash = blake3::hash(&bytes);
        if hash == self.fingerprint {
            return Ok(false);
        }
        let text = std::str::from_utf8(&bytes)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let mut tasks = codec::parse(text, Local::now().date_naive())?;
        // Keep the effective date of an undated completed line during this session.
        for (new, old) in tasks.iter_mut().zip(&self.snapshot.tasks) {
            if new.completed
                && new.completed_date.is_none()
                && old.completed
                && old.completed_date.is_none()
                && new.title == old.title
            {
                new.observed_completion = old.observed_completion;
            }
        }
        self.fingerprint = hash;
        self.snapshot.tasks = tasks;
        self.snapshot.revision += 1;
        Ok(true)
    }

    pub fn commit(&mut self, revision: u64, tasks: Vec<Task>) -> Result<Snapshot, RepositoryError> {
        if revision != self.snapshot.revision {
            return Err(RepositoryError::Stale);
        }
        if blake3::hash(&fs::read(&self.path)?) != self.fingerprint {
            self.reload()?;
            return Err(RepositoryError::Changed);
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
