use super::watch_event;
use notify::{
    Event, EventKind,
    event::{
        AccessKind, AccessMode, CreateKind, DataChange, Flag, MetadataKind, ModifyKind, RemoveKind,
        RenameMode,
    },
};
use std::path::Path;

#[test]
fn une_lecture_du_repository_ne_declenche_pas_un_nouveau_rechargement() {
    // RG-F0-10 — Étant donné le fichier surveillé et les événements de sa lecture.
    let target = Path::new("/tmp/mystline/tasks.md");
    for kind in [
        AccessKind::Any,
        AccessKind::Open(AccessMode::Any),
        AccessKind::Read,
        AccessKind::Open(AccessMode::Read),
        AccessKind::Close(AccessMode::Read),
        AccessKind::Other,
    ] {
        // Quand le worker ouvre, lit et ferme son propre fichier.
        let event = Event::new(EventKind::Access(kind)).add_path(target.into());
        // Alors aucun reload n'est demandé : la résidence ne s'autoalimente pas.
        assert!(!watch_event(&event, target), "{kind:?}");
    }
    assert!(!watch_event(
        &Event::new(EventKind::Other).add_path(target.into()),
        target
    ));
}

#[test]
fn la_fermeture_dune_ecriture_recharge_mais_pas_celle_dune_lecture() {
    // RG-F0-10 — Étant donné une sauvegarde et une simple lecture du repository.
    let target = Path::new("/tmp/mystline/tasks.md");
    let write =
        Event::new(EventKind::Access(AccessKind::Close(AccessMode::Write))).add_path(target.into());
    let read =
        Event::new(EventKind::Access(AccessKind::Close(AccessMode::Read))).add_path(target.into());
    // Quand le fichier est fermé, seule la sauvegarde doit rafraîchir le snapshot.
    assert!(watch_event(&write, target));
    assert!(!watch_event(&read, target));
}

#[test]
fn une_perte_de_notifications_recharge_meme_sans_chemin_annonce() {
    // RG-F0-10 — Étant donné un overflow signalé par le backend du watcher.
    let target = Path::new("/tmp/mystline/tasks.md");
    let overflow = Event::new(EventKind::Other).set_flag(Flag::Rescan);
    // Quand les événements détaillés ne sont plus disponibles, le fichier doit être relu.
    assert!(watch_event(&overflow, target));
    assert!(!watch_event(&Event::new(EventKind::Other), target));
}

#[test]
fn ecritures_et_remplacements_externes_continuent_de_reveiller_le_repository() {
    // RG-F0-10 — Étant donné une source et des modes de sauvegarde d'éditeurs externes.
    let target = Path::new("/tmp/mystline/tasks.md");
    for kind in [
        EventKind::Create(CreateKind::File),
        EventKind::Modify(ModifyKind::Data(DataChange::Content)),
        EventKind::Modify(ModifyKind::Metadata(MetadataKind::Permissions)),
        EventKind::Modify(ModifyKind::Name(RenameMode::To)),
        EventKind::Remove(RemoveKind::File),
    ] {
        // Quand le fichier est écrit, remplacé atomiquement ou devient inaccessible.
        let event = Event::new(kind).add_path(target.into());
        // Alors le prochain snapshot peut être rechargé et l'erreur signalée si nécessaire.
        assert!(watch_event(&event, target), "{kind:?}");
    }
    let replacement = Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::Both)))
        .add_path("/tmp/mystline/tasks.tmp".into())
        .add_path(target.into());
    assert!(watch_event(&replacement, target));
}

#[test]
fn la_sauvegarde_dun_autre_fichier_du_dossier_ne_reveille_pas_les_taches() {
    // RG-F0-10 — Étant donné config.toml et les logs dans le même répertoire surveillé.
    let target = Path::new("/tmp/mystline/tasks.md");
    for path in [
        "/tmp/mystline/config.toml",
        "/tmp/mystline/logs/mystline.log",
        "/tmp/mystline/tasks.tmp",
    ] {
        // Quand un fichier voisin est créé ou modifié.
        let event = Event::new(EventKind::Modify(ModifyKind::Data(DataChange::Content)))
            .add_path(path.into());
        // Alors il ne provoque pas de lecture de tasks.md.
        assert!(!watch_event(&event, target));
    }
}
