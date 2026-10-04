//! Le snapshot affiché est optimiste ; seul le worker touche le repository.
use mystline::{
    application::{self, TaskCommand},
    repository::{Snapshot, TaskRepository},
};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceRevision {
    generation: u64,
    revision: u64,
}

impl SourceRevision {
    pub fn with_revision(self, revision: u64) -> Self {
        Self { revision, ..self }
    }
    #[cfg(test)]
    pub fn for_test(revision: u64) -> Self {
        Self {
            generation: 0,
            revision,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationKind {
    Write,
    Reload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OperationToken {
    generation: u64,
    sequence: u64,
}

pub struct TaskStore {
    repo: Arc<Mutex<TaskRepository>>,
    pub snapshot: Snapshot,
    confirmed: Snapshot,
    generation: u64,
    sequence: u64,
    pending: Option<OperationToken>,
}

pub struct Job {
    repo: Arc<Mutex<TaskRepository>>,
    token: OperationToken,
    kind: OperationKind,
    revision: u64,
    command: Option<TaskCommand>,
    confirmed: Snapshot,
}

#[derive(Debug, Clone)]
pub struct JobResult {
    token: OperationToken,
    pub kind: OperationKind,
    pub snapshot: Snapshot,
    pub error: Option<String>,
}

impl TaskStore {
    pub fn new(repo: TaskRepository) -> Self {
        let snapshot = repo.snapshot();
        Self {
            repo: Arc::new(Mutex::new(repo)),
            confirmed: snapshot.clone(),
            snapshot,
            generation: 0,
            sequence: 0,
            pending: None,
        }
    }

    pub fn source_revision(&self) -> SourceRevision {
        SourceRevision {
            generation: self.generation,
            revision: self.snapshot.revision,
        }
    }

    pub fn is_busy(&self) -> bool {
        self.pending.is_some()
    }

    pub fn begin_write(
        &mut self,
        source: SourceRevision,
        command: TaskCommand,
    ) -> Result<Job, String> {
        if source != self.source_revision() {
            return Err("La liste ou le fichier source a changé. Reprendre l'action depuis la liste actuelle.".into());
        }
        if self.is_busy() {
            return Err("Un enregistrement ou un rechargement est en cours. Réessayer après sa confirmation.".into());
        }
        let proposed = application::propose_task_change(&self.confirmed, source.revision, &command)
            .map_err(|error| error.to_string())?;
        let job = self.job(OperationKind::Write, Some(command));
        self.snapshot = proposed;
        Ok(job)
    }

    pub fn begin_reload(&mut self) -> Option<Job> {
        if self.is_busy() {
            return None;
        }
        Some(self.job(OperationKind::Reload, None))
    }

    fn job(&mut self, kind: OperationKind, command: Option<TaskCommand>) -> Job {
        self.sequence += 1;
        let token = OperationToken {
            generation: self.generation,
            sequence: self.sequence,
        };
        self.pending = Some(token);
        Job {
            repo: self.repo.clone(),
            token,
            kind,
            revision: self.confirmed.revision,
            command,
            confirmed: self.confirmed.clone(),
        }
    }

    /// Un résultat retardé d'une ancienne source ne remplace jamais la nouvelle.
    pub fn finish(&mut self, result: &JobResult) -> bool {
        if self.pending != Some(result.token) {
            return false;
        }
        self.pending = None;
        self.confirmed = result.snapshot.clone();
        self.snapshot = result.snapshot.clone();
        true
    }

    pub fn replace_source(&mut self, repo: TaskRepository) {
        self.generation += 1;
        self.confirmed = repo.snapshot();
        self.repo = Arc::new(Mutex::new(repo));
        self.snapshot = self.confirmed.clone();
        self.pending = None;
    }
}

impl Job {
    pub(super) fn run(self) -> JobResult {
        let mut result = JobResult {
            token: self.token,
            kind: self.kind,
            snapshot: self.confirmed,
            error: None,
        };
        match self.repo.lock() {
            Ok(mut repo) => {
                result.error = match self.command {
                    Some(command) => {
                        application::apply_task_command(&mut repo, self.revision, &command)
                            .err()
                            .map(|error| error.to_string())
                    }
                    None => repo.reload().err().map(|error| error.to_string()),
                };
                result.snapshot = repo.snapshot();
            }
            Err(_) => result.error = Some("Le stockage n'est plus disponible.".into()),
        }
        result
    }

    pub async fn perform(self) -> JobResult {
        let fallback = JobResult {
            token: self.token,
            kind: self.kind,
            snapshot: self.confirmed.clone(),
            error: None,
        };
        match tokio::task::spawn_blocking(move || self.run()).await {
            Ok(result) => result,
            Err(error) => JobResult {
                error: Some(format!("Échec du stockage : {error}")),
                ..fallback
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use mystline::application::TaskInput;
    use std::fs;

    fn day(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, day).unwrap()
    }

    fn repository(path: &std::path::Path, text: &str) -> TaskRepository {
        fs::write(path, text).unwrap();
        TaskRepository::open_at(path.to_owned(), false, day(9)).unwrap()
    }

    #[test]
    fn les_quatre_actions_saffichent_avant_le_commit_puis_sont_confirmees() {
        // Architecture §9.3/26/28/29 — Étant donné un fichier confirmé et quatre intentions.
        for command in [
            TaskCommand::Create(TaskInput {
                title: "Nouvelle".into(),
                ..TaskInput::default()
            }),
            TaskCommand::Update {
                position: 0,
                input: TaskInput {
                    title: "Modifiée".into(),
                    ..TaskInput::default()
                },
            },
            TaskCommand::Complete {
                position: 0,
                today: day(9),
            },
            TaskCommand::Reorder { from: 0, to: 1 },
        ] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("tasks.md");
            let original = "- [ ] A #mission @planned(2026-10-09)\n- [ ] B\n";
            let mut store = TaskStore::new(repository(&path, original));
            let confirmed = store.snapshot.clone();
            // Quand l'UI accepte l'intention, avant d'exécuter le worker.
            let job = store
                .begin_write(store.source_revision(), command.clone())
                .unwrap();
            // Alors la modification est déjà visible, tandis que fichier et cache restent confirmés.
            match command {
                TaskCommand::Create(_) => assert_eq!(store.snapshot.tasks[2].title, "Nouvelle"),
                TaskCommand::Update { .. } => assert_eq!(store.snapshot.tasks[0].title, "Modifiée"),
                TaskCommand::Complete { .. } => assert!(store.snapshot.tasks[0].completed),
                TaskCommand::Reorder { .. } => assert_eq!(store.snapshot.tasks[0].title, "B"),
            }
            assert_eq!(fs::read_to_string(&path).unwrap(), original);
            assert_eq!(store.confirmed.tasks, confirmed.tasks);
            assert_eq!(store.repo.lock().unwrap().snapshot().tasks, confirmed.tasks);
            let result = job.run();
            assert!(result.error.is_none());
            assert!(store.finish(&result));
            assert_eq!(store.snapshot.revision, confirmed.revision + 1);
            assert!(!store.is_busy());
            assert_eq!(
                TaskRepository::open_at(path, false, day(10))
                    .unwrap()
                    .snapshot()
                    .tasks,
                store.snapshot.tasks
            );
        }
    }

    #[test]
    fn un_echec_de_sauvegarde_annule_la_completion_optimiste() {
        // Architecture §9.3 — Étant donné une tâche active et une complétion déjà affichée.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.md");
        let mut store = TaskStore::new(repository(&path, "- [ ] A\n"));
        let confirmed = store.snapshot.clone();
        let job = store
            .begin_write(
                store.source_revision(),
                TaskCommand::Complete {
                    position: 0,
                    today: day(9),
                },
            )
            .unwrap();
        assert!(store.snapshot.tasks[0].completed);
        // Quand le worker découvre que le fichier est devenu inaccessible.
        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();
        let result = job.run();
        assert!(result.error.is_some());
        store.finish(&result);
        // Alors la tâche active et la révision confirmée sont restaurées.
        assert_eq!(store.snapshot.tasks, confirmed.tasks);
        assert_eq!(store.snapshot.revision, confirmed.revision);
        assert!(!store.is_busy());
    }

    #[test]
    fn un_conflit_remplace_la_proposition_par_la_version_exterieure() {
        // Architecture §16 — Étant donné une permutation optimiste avant son écriture.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.md");
        let mut store = TaskStore::new(repository(&path, "- [ ] A\n- [ ] B\n"));
        let job = store
            .begin_write(
                store.source_revision(),
                TaskCommand::Reorder { from: 0, to: 1 },
            )
            .unwrap();
        assert_eq!(store.snapshot.tasks[0].title, "B");
        // Quand une édition extérieure survient avant le worker.
        let external = "- [ ] Extérieure\n";
        fs::write(&path, external).unwrap();
        let result = job.run();
        assert!(result.error.is_some());
        store.finish(&result);
        // Alors la version extérieure reste intacte et remplace l'affichage optimiste.
        assert_eq!(store.snapshot.tasks[0].title, "Extérieure");
        assert_eq!(fs::read_to_string(path).unwrap(), external);
    }

    #[test]
    fn une_operation_en_cours_empeche_les_ecritures_concurrentes() {
        // Architecture §18 — Étant donné un enregistrement non encore confirmé.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.md");
        let mut store = TaskStore::new(repository(&path, "- [ ] A\n- [ ] B\n"));
        let source = store.source_revision();
        let job = store
            .begin_write(
                source,
                TaskCommand::Complete {
                    position: 0,
                    today: day(9),
                },
            )
            .unwrap();
        // Quand une autre complétion ou un rechargement est demandé immédiatement.
        assert!(
            store
                .begin_write(
                    source,
                    TaskCommand::Complete {
                        position: 1,
                        today: day(9)
                    }
                )
                .is_err()
        );
        assert!(store.begin_reload().is_none());
        // Alors le premier enregistrement reste seul en cours ; le second peut être repris ensuite.
        let result = job.run();
        store.finish(&result);
        assert!(!store.snapshot.tasks[1].completed);
        let job = store
            .begin_write(
                store.source_revision(),
                TaskCommand::Complete {
                    position: 1,
                    today: day(9),
                },
            )
            .unwrap();
        store.finish(&job.run());
        assert!(store.snapshot.tasks.iter().all(|task| task.completed));
    }

    #[test]
    fn une_ancienne_commande_ne_coche_pas_le_nouveau_fichier_a_revision_egale() {
        // Architecture §15/20 — Étant donné deux fichiers à la même révision runtime.
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("first.md");
        let second = dir.path().join("second.md");
        let mut store = TaskStore::new(repository(&first, "- [ ] Première\n"));
        let old_source = store.source_revision();
        // Quand les paramètres remplacent la source, avant réception d'un ancien clic.
        store.replace_source(repository(&second, "- [ ] Autre source\n"));
        assert_eq!(old_source.revision, store.source_revision().revision);
        assert!(
            store
                .begin_write(
                    old_source,
                    TaskCommand::Complete {
                        position: 0,
                        today: day(9)
                    }
                )
                .is_err()
        );
        // Alors aucune ligne du nouveau fichier n'est cochée ou réécrite.
        assert_eq!(fs::read_to_string(second).unwrap(), "- [ ] Autre source\n");
        assert!(!store.snapshot.tasks[0].completed);
        assert!(!store.is_busy());
    }

    #[test]
    fn une_reponse_de_lancienne_source_ne_remplace_pas_la_nouvelle_operation() {
        // Architecture §15/20 — Étant donné une ancienne écriture dont le retour est retardé.
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("first.md");
        let second = dir.path().join("second.md");
        let mut store = TaskStore::new(repository(&first, "- [ ] Ancienne\n"));
        let old = store
            .begin_write(
                store.source_revision(),
                TaskCommand::Complete {
                    position: 0,
                    today: day(9),
                },
            )
            .unwrap();
        store.replace_source(repository(&second, "- [ ] Nouvelle\n"));
        let new = store
            .begin_write(
                store.source_revision(),
                TaskCommand::Complete {
                    position: 0,
                    today: day(9),
                },
            )
            .unwrap();
        // Quand l'ancienne réponse arrive pendant la nouvelle opération.
        assert!(!store.finish(&old.run()));
        // Alors elle ne touche ni la nouvelle proposition, ni son état d'enregistrement.
        assert!(store.is_busy());
        assert_eq!(store.snapshot.tasks[0].title, "Nouvelle");
        assert!(store.snapshot.tasks[0].completed);
        assert!(store.finish(&new.run()));
        assert_eq!(store.snapshot.tasks[0].title, "Nouvelle");
    }

    #[test]
    fn le_worker_bloque_sur_le_disque_laisse_lexecuteur_ui_progresser() {
        // Architecture §9.3 — Étant donné le worker empêché d'accéder au repository.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.md");
        let mut store = TaskStore::new(repository(&path, "- [ ] A\n"));
        let job = store
            .begin_write(
                store.source_revision(),
                TaskCommand::Complete {
                    position: 0,
                    today: day(9),
                },
            )
            .unwrap();
        let repo = store.repo.clone();
        let (locked_tx, locked_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let blocker = std::thread::spawn(move || {
            let _lock = repo.lock().unwrap();
            locked_tx.send(()).unwrap();
            let _ = release_rx.recv_timeout(std::time::Duration::from_secs(2));
        });
        locked_rx.recv().unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            // Quand l'opération asynchrone démarre, un réveil de l'exécuteur reste possible.
            let handle = tokio::spawn(job.perform());
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            assert!(!handle.is_finished());
            assert!(store.snapshot.tasks[0].completed);
            release_tx.send(()).unwrap();
            let result = handle.await.unwrap();
            assert!(result.error.is_none());
            store.finish(&result);
        });
        // Alors le commit se termine après libération du disque, sans avoir bloqué l'exécuteur.
        blocker.join().unwrap();
        assert_eq!(store.snapshot.revision, 1);
    }

    #[test]
    fn le_rechargement_du_watcher_reste_en_arriere_plan_et_preserve_un_fichier_invalide() {
        // Architecture §12/13 — Étant donné le dernier snapshot confirmé.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.md");
        let mut store = TaskStore::new(repository(&path, "- [ ] A\n"));
        // Quand le watcher demande une lecture d'un fichier temporairement invalide.
        fs::write(&path, "Sauvegarde en cours").unwrap();
        let job = store.begin_reload().unwrap();
        assert_eq!(store.snapshot.tasks[0].title, "A");
        let result = job.run();
        assert!(result.error.is_some());
        store.finish(&result);
        // Alors le snapshot valide est conservé sans normaliser le fichier.
        assert_eq!(store.snapshot.tasks[0].title, "A");
        assert_eq!(fs::read_to_string(path).unwrap(), "Sauvegarde en cours");
    }
}
