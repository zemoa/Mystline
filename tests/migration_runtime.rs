//! Parcours de coordination async réutilisés par l'IPC Tauri.
use chrono::NaiveDate;
use mystline::{
    application::{TaskCommand, TaskInput},
    repository::TaskRepository,
    runtime::coordinator::{Coordinator, Creation, Operation},
};
use std::fs;

fn day() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, 9).unwrap()
}

fn capture(origin: u64, draft: &str) -> Creation<u64> {
    Creation {
        origin,
        draft: draft.into(),
        input: TaskInput {
            title: draft.into(),
            ..TaskInput::default()
        },
    }
}

fn finish(coordinator: &mut Coordinator<u64>, operation: Operation<u64>) -> Option<String> {
    let result = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(operation.job.perform());
    assert!(coordinator.finish(&result));
    result.error
}

#[test]
fn une_capture_en_conflit_ne_confirme_pas_son_brouillon_et_la_file_continue() {
    // RG-F0-09/10, F1-03 — Étant donné une capture et une seconde origine en attente.
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("tasks.md");
    let mut coordinator =
        Coordinator::new(TaskRepository::open_at(path.clone(), true, day()).unwrap());
    let operation = coordinator.create(capture(1, "Refusée")).unwrap().unwrap();
    assert!(
        coordinator
            .create(capture(2, "Après conflit"))
            .unwrap()
            .is_none()
    );
    assert!(
        coordinator
            .create(capture(2, "Après conflit"))
            .unwrap()
            .is_none()
    );
    // Quand une édition extérieure précède le worker et un reload est demandé pendant l'écriture.
    let external = "- [ ] Édition extérieure #mission\n";
    fs::write(&path, external).unwrap();
    assert!(coordinator.reload().is_none());
    assert!(finish(&mut coordinator, operation).is_some());
    // Alors la capture n'est pas confirmée ; la version externe est conservée.
    assert_eq!(fs::read_to_string(&path).unwrap(), external);
    assert_eq!(coordinator.tasks.snapshot.tasks.len(), 1);
    let queued = coordinator.next_operation().unwrap().unwrap();
    assert_eq!(queued.creation.as_ref().unwrap().origin, 2);
    assert!(finish(&mut coordinator, queued).is_none());
    // Et la capture suivante est ajoutée une seule fois à cette version, suivie du reload.
    let reload = coordinator.next_operation().unwrap().unwrap();
    assert!(reload.creation.is_none());
    assert!(finish(&mut coordinator, reload).is_none());
    assert!(coordinator.next_operation().unwrap().is_none());
    let reopened = TaskRepository::open_at(path, false, day())
        .unwrap()
        .snapshot();
    assert_eq!(
        reopened
            .tasks
            .iter()
            .map(|task| task.title.as_str())
            .collect::<Vec<_>>(),
        ["Édition extérieure", "Après conflit"]
    );
}

#[test]
fn un_resultat_deja_traite_ne_termine_pas_la_capture_suivante() {
    // Architecture §9.3 — Étant donné une écriture terminée et une capture FIFO suivante.
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("tasks.md");
    let mut coordinator =
        Coordinator::new(TaskRepository::open_at(path.clone(), true, day()).unwrap());
    let first = coordinator.create(capture(1, "Première")).unwrap().unwrap();
    coordinator.create(capture(2, "Seconde")).unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let first_result = runtime.block_on(first.job.perform());
    assert!(coordinator.finish(&first_result));
    let second = coordinator.next_operation().unwrap().unwrap();
    // Quand une réponse dupliquée de la première opération arrive.
    assert!(!coordinator.finish(&first_result));
    // Alors elle ne libère pas l'opération active ni n'autorise un changement de source.
    assert!(coordinator.tasks.is_busy());
    assert!(
        coordinator
            .replace_source(
                TaskRepository::open_at(directory.path().join("other.md"), true, day()).unwrap()
            )
            .is_err()
    );
    assert!(finish(&mut coordinator, second).is_none());
    assert_eq!(
        fs::read_to_string(path).unwrap(),
        "- [ ] Première\n- [ ] Seconde\n"
    );
}

#[test]
fn un_echange_source_a_revision_identique_refuse_la_commande_de_lancien_panneau() {
    // RG-F0-04/09 — Étant donné deux sources à même révision mais contenus différents.
    let directory = tempfile::tempdir().unwrap();
    let old_path = directory.path().join("first.md");
    let new_path = directory.path().join("second.md");
    fs::write(&old_path, "- [ ] Ancienne\n").unwrap();
    fs::write(&new_path, "- [ ] Nouvelle\n").unwrap();
    let mut coordinator =
        Coordinator::<u64>::new(TaskRepository::open_at(old_path.clone(), false, day()).unwrap());
    let old_source = coordinator.tasks.source_revision();
    // Quand une ancienne commande tente de modifier la nouvelle source.
    coordinator
        .replace_source(TaskRepository::open_at(new_path.clone(), false, day()).unwrap())
        .unwrap();
    assert!(
        coordinator
            .write(
                old_source,
                TaskCommand::Complete {
                    position: 0,
                    today: day()
                }
            )
            .is_err()
    );
    // Alors aucune opération ne commence et les deux fichiers restent intacts.
    assert!(!coordinator.tasks.is_busy());
    assert_eq!(fs::read_to_string(old_path).unwrap(), "- [ ] Ancienne\n");
    assert_eq!(fs::read_to_string(new_path).unwrap(), "- [ ] Nouvelle\n");
}
