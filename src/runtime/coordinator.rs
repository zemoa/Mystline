use super::task_store::{Job, JobResult, SourceRevision, TaskStore};
use crate::{
    application::{TaskCommand, TaskInput},
    repository::TaskRepository,
};
use std::collections::VecDeque;

/// Conserve le brouillon et sa surface d'origine jusqu'à la confirmation.
pub struct Creation<Origin> {
    pub origin: Origin,
    pub draft: String,
    pub input: TaskInput,
}

pub struct Operation<Origin> {
    pub job: Job,
    pub creation: Option<Creation<Origin>>,
}

/// Une opération de stockage à la fois ; les captures attendent en FIFO.
pub struct Coordinator<Origin> {
    pub tasks: TaskStore,
    queue: VecDeque<Creation<Origin>>,
    active_create: Option<Origin>,
    reload_pending: bool,
}

impl<Origin: Clone + PartialEq> Coordinator<Origin> {
    pub fn open(
        path: std::path::PathBuf,
        create: bool,
    ) -> Result<Self, crate::repository::RepositoryError> {
        TaskRepository::open(path, create).map(Self::new)
    }
    pub fn new(repo: TaskRepository) -> Self {
        Self {
            tasks: TaskStore::new(repo),
            queue: VecDeque::new(),
            active_create: None,
            reload_pending: false,
        }
    }

    pub fn create(
        &mut self,
        creation: Creation<Origin>,
    ) -> Result<Option<Operation<Origin>>, String> {
        if self.active_create.as_ref() == Some(&creation.origin)
            || self
                .queue
                .iter()
                .any(|entry| entry.origin == creation.origin)
        {
            return Ok(None);
        }
        if self.tasks.is_busy() {
            self.queue.push_back(creation);
            return Ok(None);
        }
        self.start_create(creation).map(Some)
    }

    fn start_create(&mut self, creation: Creation<Origin>) -> Result<Operation<Origin>, String> {
        let job = self.tasks.begin_write(
            self.tasks.source_revision(),
            TaskCommand::Create(creation.input.clone()),
        )?;
        self.active_create = Some(creation.origin.clone());
        Ok(Operation {
            job,
            creation: Some(creation),
        })
    }

    pub fn write(
        &mut self,
        source: SourceRevision,
        command: TaskCommand,
    ) -> Result<Operation<Origin>, String> {
        let job = self.tasks.begin_write(source, command)?;
        Ok(Operation {
            job,
            creation: None,
        })
    }

    pub fn reload(&mut self) -> Option<Operation<Origin>> {
        match self.tasks.begin_reload() {
            Some(job) => {
                self.reload_pending = false;
                Some(Operation {
                    job,
                    creation: None,
                })
            }
            None => {
                self.reload_pending = true;
                None
            }
        }
    }

    pub fn finish(&mut self, result: &JobResult) -> bool {
        if !self.tasks.finish(result) {
            return false;
        }
        self.active_create = None;
        true
    }

    /// Priorité aux créations reçues, puis au dernier rechargement demandé.
    pub fn next_operation(&mut self) -> Result<Option<Operation<Origin>>, String> {
        if self.tasks.is_busy() {
            return Ok(None);
        }
        if let Some(creation) = self.queue.pop_front() {
            return self.start_create(creation).map(Some);
        }
        Ok(if self.reload_pending {
            self.reload()
        } else {
            None
        })
    }

    pub fn replace_source(&mut self, repo: TaskRepository) -> Result<(), String> {
        if self.tasks.is_busy() || !self.queue.is_empty() {
            return Err("Attendre la fin des enregistrements avant de changer de fichier.".into());
        }
        self.tasks.replace_source(repo);
        self.reload_pending = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn creation(origin: u8, title: &str) -> Creation<u8> {
        Creation {
            origin,
            draft: title.into(),
            input: TaskInput {
                title: title.into(),
                ..TaskInput::default()
            },
        }
    }

    #[test]
    fn les_captures_attendent_le_rechargement_puis_sont_persistees_dans_lordre() {
        // Architecture §9.3 — Étant donné un rechargement en cours.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.md");
        let mut runtime = Coordinator::new(TaskRepository::open(path.clone(), true).unwrap());
        let reload = runtime.reload().unwrap();
        // Quand deux captures sont reçues, dont une touche Entrée répétée.
        assert!(runtime.create(creation(1, "Première")).unwrap().is_none());
        assert!(runtime.create(creation(1, "Première")).unwrap().is_none());
        assert!(runtime.create(creation(2, "Seconde")).unwrap().is_none());
        runtime.finish(&reload.job.run());
        for expected in ["Première", "Seconde"] {
            let operation = runtime.next_operation().unwrap().unwrap();
            assert_eq!(operation.creation.as_ref().unwrap().draft, expected);
            runtime.finish(&operation.job.run());
        }
        // Alors chaque tâche est persistée une fois, dans l'ordre de réception.
        assert_eq!(
            fs::read_to_string(path).unwrap(),
            "- [ ] Première\n- [ ] Seconde\n"
        );
        assert!(runtime.next_operation().unwrap().is_none());
    }

    #[test]
    fn une_capture_en_attente_ne_peut_pas_basculer_sur_un_autre_fichier() {
        // RG-F0-04 — Étant donné une capture en attente derrière un rechargement.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("first.md");
        let mut runtime = Coordinator::new(TaskRepository::open(path.clone(), true).unwrap());
        let reload = runtime.reload().unwrap();
        runtime
            .create(creation(1, "Dans le premier fichier"))
            .unwrap();
        runtime.finish(&reload.job.run());
        // Quand un changement de source précède le traitement de la capture.
        let other = dir.path().join("second.md");
        assert!(
            runtime
                .replace_source(TaskRepository::open(other.clone(), true).unwrap())
                .is_err()
        );
        let operation = runtime.next_operation().unwrap().unwrap();
        runtime.finish(&operation.job.run());
        // Alors le changement est refusé et le brouillon reste dans sa source.
        assert!(
            fs::read_to_string(path)
                .unwrap()
                .contains("Dans le premier fichier")
        );
        assert_eq!(fs::read_to_string(other).unwrap(), "");
    }
}
