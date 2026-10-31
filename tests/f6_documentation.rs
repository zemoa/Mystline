//! Recette F6 validée : parcours métier avec dates fixes et véritable fichier Markdown.
use chrono::NaiveDate;
use mystline::{
    application::{self, TaskCommand},
    presentation::{
        input::{format_edit, parse_edit_input, parse_input},
        panel::{PanelMode, PanelState, SortRank},
    },
    repository::{Snapshot, TaskRepository},
    runtime::task_store::TaskStore,
};
use std::fs;

fn date(raw: &str) -> NaiveDate {
    raw.parse().unwrap()
}

struct Fixture {
    _dir: tempfile::TempDir,
    repo: TaskRepository,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let repo =
            TaskRepository::open_at(dir.path().join("tasks.md"), true, date("2026-11-01")).unwrap();
        Self { _dir: dir, repo }
    }
    fn create(&mut self, raw: &str, today: &str) -> Snapshot {
        application::create_task_from_input(&mut self.repo, &parse_input(raw, date(today)).unwrap())
            .unwrap()
    }
    fn complete(&mut self, position: usize, today: &str) -> Snapshot {
        let revision = self.repo.snapshot().revision;
        application::complete_task(&mut self.repo, revision, position, date(today)).unwrap()
    }
    fn edit(&mut self, position: usize, raw: &str) -> Snapshot {
        let revision = self.repo.snapshot().revision;
        application::update_task(
            &mut self.repo,
            revision,
            position,
            &parse_edit_input(raw, date("2026-11-01")).unwrap(),
        )
        .unwrap()
    }
    fn reopen(&mut self, position: usize) -> Snapshot {
        let revision = self.repo.snapshot().revision;
        application::reopen_task(&mut self.repo, revision, position).unwrap()
    }
    fn text(&self) -> String {
        fs::read_to_string(self.repo.path()).unwrap()
    }
    fn restart(&mut self, today: &str) {
        let before = self.repo.snapshot();
        self.repo =
            TaskRepository::open_at(self.repo.path().to_owned(), false, date(today)).unwrap();
        assert_eq!(self.repo.snapshot().tasks, before.tasks);
    }
}

#[test]
fn les_quatre_frequences_proposent_la_premiere_date_compatible_aujourdhui_compris() {
    // RG-F6-03/06/15 — Étant donné dimanche 1er novembre, quand on capture sans /p.
    for (rule, expected) in [
        ("jour", "2026-11-01"),
        ("30j", "2026-11-01"),
        ("dimanche", "2026-11-01"),
        ("lundi,vendredi", "2026-11-02"),
        ("mois:31", "2026-11-30"),
    ] {
        let mut f = Fixture::new();
        let snapshot = f.create(&format!("Revue /r {rule}"), "2026-11-01");
        // Alors une seule tâche persistée porte la date proposée.
        assert_eq!(snapshot.tasks.len(), 1);
        assert_eq!(snapshot.tasks[0].planned, Some(date(expected)));
        f.restart("2026-11-02");
    }
}

#[test]
fn les_saisies_invalides_et_dates_incompatibles_ne_creent_aucune_tache() {
    // RG-F6-03/15 — Quand une commande ou sa première date est invalide.
    let f = Fixture::new();
    for raw in [
        "Revue /r",
        "Revue /r 0j",
        "Revue /r -1j",
        "Revue /r 1.5j",
        "Revue /r inconnu",
        "Revue /r mois:0",
        "Revue /r mois:32",
        "Revue /r jour /r vendredi",
        "Revue /r vendredi /p 2026-11-02",
        "Revue /r mois:31 /p 2026-11-29",
    ] {
        assert!(parse_input(raw, date("2026-11-01")).is_err(), "{raw}");
    }
    // Alors le fichier et l'état confirmé restent vides.
    assert!(f.repo.snapshot().tasks.is_empty());
    assert!(f.text().is_empty());
}

#[test]
fn les_commandes_echappees_restent_litterales_apres_edition_et_redemarrage() {
    // Syntaxe F6 — Étant donné un titre avec /r et des antislashs littéraux.
    let mut f = Fixture::new();
    let snapshot = f.create(r"Relire \/r vendredi \\serveur #perso", "2026-11-01");
    assert!(snapshot.tasks[0].recurrence.is_none());
    let draft = format_edit(&snapshot.tasks[0]);
    assert_eq!(f.edit(0, &draft).tasks, snapshot.tasks);
    f.restart("2026-11-02");
}

#[test]
fn le_cochage_conserve_lhistorique_et_affiche_immediatement_la_suivante() {
    // RG-F6-02/04 — Étant donné une occurrence quotidienne.
    let mut f = Fixture::new();
    f.create("Revue /r jour", "2026-11-01");
    let snapshot = f.complete(0, "2026-11-01");
    let mut panel = PanelState::new(date("2026-11-01"));
    assert_eq!(panel.rows(&snapshot).len(), 2);
    assert!(snapshot.tasks[0].completed);
    assert_eq!(snapshot.tasks[0].completed_date, Some(date("2026-11-01")));
    assert_eq!(snapshot.tasks.iter().filter(|t| !t.completed).count(), 1);
    assert_eq!(snapshot.tasks[1].planned, Some(date("2026-11-02")));
    // Alors le lendemain seule la suivante est au panneau, et l'ancienne reste dans l'historique.
    panel.today = date("2026-11-02");
    assert_eq!(panel.indices(&snapshot), [1]);
    panel.mode = PanelMode::History;
    assert_eq!(panel.indices(&snapshot), [0]);
    f.restart("2026-11-02");
}

#[test]
fn les_trente_jours_ne_derivent_pas_avec_le_retard_et_ne_sont_pas_un_mois() {
    // RG-F6-01/05/06 — Quand le 1er novembre est réalisé le 5.
    let mut f = Fixture::new();
    f.create("Revue /p 2026-11-01 /r 30j", "2026-11-01");
    assert_eq!(
        f.complete(0, "2026-11-05").tasks[1].planned,
        Some(date("2026-12-01"))
    );
    assert_eq!(
        f.complete(1, "2026-12-01").tasks[2].planned,
        Some(date("2026-12-31"))
    );
    f.restart("2026-12-02");
}

#[test]
fn les_vendredis_manques_ne_saccumulent_pas_et_lanticipation_garde_la_date_theorique() {
    // RG-F6-05/13 — Étant donné vendredi 6 novembre, deux réalisations possibles.
    for (done, next) in [("2026-11-23", "2026-11-27"), ("2026-11-05", "2026-11-13")] {
        let mut f = Fixture::new();
        f.create("Revue /p 2026-11-06 /r vendredi", "2026-11-01");
        let snapshot = f.complete(0, done);
        assert_eq!(snapshot.tasks.len(), 2);
        assert_eq!(snapshot.tasks[0].completed_date, Some(date(done)));
        assert_eq!(snapshot.tasks[1].planned, Some(date(next)));
        f.restart(done);
    }
}

#[test]
fn les_mois_courts_conservent_le_jour_initial_y_compris_en_annee_bissextile() {
    // RG-F6-07 — Quand on réalise janvier puis février dans les deux types d'années.
    for (first, february, march) in [
        ("2027-01-31", "2027-02-28", "2027-03-31"),
        ("2028-01-31", "2028-02-29", "2028-03-31"),
    ] {
        let mut f = Fixture::new();
        f.create(&format!("Revue /p {first} /r mois:31"), first);
        assert_eq!(f.complete(0, first).tasks[1].planned, Some(date(february)));
        assert_eq!(f.complete(1, february).tasks[2].planned, Some(date(march)));
        f.restart(march);
    }
}

#[test]
fn plusieurs_jours_hebdomadaires_suivent_un_seul_calendrier() {
    // RG-F6-02/06 — Quand on coche lundi, puis vendredi.
    let mut f = Fixture::new();
    f.create("Revue /r lundi,vendredi /p 2026-11-02", "2026-11-01");
    assert_eq!(
        f.complete(0, "2026-11-02").tasks[1].planned,
        Some(date("2026-11-06"))
    );
    let snapshot = f.complete(1, "2026-11-06");
    assert_eq!(snapshot.tasks[2].planned, Some(date("2026-11-09")));
    assert_eq!(snapshot.tasks.iter().filter(|t| !t.completed).count(), 1);
}

#[test]
fn le_report_ne_deplace_ni_le_calendrier_ni_la_deadline() {
    // RG-F6-08/10 — Étant donné vendredi avec deadline lundi, reporté au lundi.
    let mut f = Fixture::new();
    f.create(
        "Revue /r vendredi /p 2026-11-06 /d 2026-11-09",
        "2026-11-01",
    );
    let reported = f.edit(0, "Revue /r vendredi /p 2026-11-09 /d 2026-11-09");
    assert_eq!(reported.tasks[0].deadline, Some(date("2026-11-09")));
    f.restart("2026-11-09");
    let snapshot = f.complete(0, "2026-11-09");
    assert_eq!(snapshot.tasks[1].planned, Some(date("2026-11-13")));
    assert_eq!(snapshot.tasks[1].deadline, Some(date("2026-11-16")));
}

#[test]
fn les_metadonnees_et_filtres_ne_dupliquent_pas_les_occurrences_et_lordre_reste_manuel() {
    // RG-F6-09 — Étant donné deux tâches créées séparément avec le même titre.
    let mut f = Fixture::new();
    for _ in 0..2 {
        f.create(
            "Revue /r vendredi /p 2026-11-06 /d 2026-11-05 #perso #mission",
            "2026-11-01",
        );
    }
    let before = f.repo.snapshot();
    assert_ne!(before.tasks[0].occurrence, before.tasks[1].occurrence);
    let mut panel = PanelState::new(date("2026-11-06"));
    panel.query = "revue".into();
    panel.tag = Some("perso".into());
    let rows = panel.rows(&before);
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|r| r.rank == SortRank::Overdue));
    let to = panel.neighbor(&before, 0, true).unwrap();
    let after = application::reorder_tasks(&mut f.repo, before.revision, 0, to).unwrap();
    assert_eq!(after.tasks[0], before.tasks[1]);
    f.restart("2026-11-06");
}

#[test]
fn modifier_titre_tags_frequence_et_deadline_ne_modifie_pas_lhistorique() {
    // RG-F6-11/16 — Étant donné une ancienne occurrence et une suivante en attente.
    let mut f = Fixture::new();
    f.create("Revue /r vendredi /p 2026-11-06 #ancien", "2026-11-01");
    let old = f.complete(0, "2026-11-06").tasks[0].clone();
    let edited = f.edit(1, "Bilan /r lundi /p 2026-11-16 /d 2026-11-18 #nouveau");
    assert_eq!(edited.tasks.len(), 2);
    assert_eq!(edited.tasks[0], old);
    f.restart("2026-11-16");
    let next = f.complete(1, "2026-11-16");
    assert_eq!(next.tasks[2].title, "Bilan");
    assert_eq!(next.tasks[2].tags, ["nouveau"]);
    assert_eq!(next.tasks[2].planned, Some(date("2026-11-23")));
    assert_eq!(next.tasks[2].deadline, Some(date("2026-11-25")));
    // Modifier la deadline après report se réfère à la date théorique, puis son retrait se propage.
    f.edit(2, "Bilan /r lundi /p 2026-11-24 /d 2026-11-27 #nouveau");
    let next = f.complete(2, "2026-11-24");
    assert_eq!(next.tasks[3].deadline, Some(date("2026-12-04")));
    f.edit(3, "Bilan /r lundi /p 2026-11-30 #nouveau");
    assert_eq!(f.complete(3, "2026-11-30").tasks[4].deadline, None);
}

#[test]
fn arreter_la_repetition_conserve_la_tache_et_ne_genere_plus_de_suivante() {
    // RG-F6-12/17 — Quand « Ne plus répéter » retire /r.
    let mut f = Fixture::new();
    let initial = f.create(
        "Revue /r vendredi /p 2026-11-06 /d 2026-11-09 #perso",
        "2026-11-01",
    );
    let after = f.edit(0, "Revue /p 2026-11-06 /d 2026-11-09 #perso");
    assert_eq!(after.tasks[0].planned, initial.tasks[0].planned);
    assert_eq!(after.tasks[0].deadline, initial.tasks[0].deadline);
    assert_eq!(after.tasks[0].tags, initial.tasks[0].tags);
    f.restart("2026-11-06");
    assert_eq!(f.complete(0, "2026-11-06").tasks.len(), 1);
}

#[test]
fn decocher_retire_la_suivante_meme_modifiee_ou_devenue_ponctuelle() {
    // RG-F6-14 — Quand la suivante est modifiée puis le cochage annulé.
    for stop in [false, true] {
        let mut f = Fixture::new();
        let initial = f.create("Revue /r jour /p 2026-11-01", "2026-11-01");
        f.complete(0, "2026-11-01");
        f.edit(
            1,
            if stop {
                "Modifiée /p 2026-11-02"
            } else {
                "Modifiée /p 2026-11-02 /r jour"
            },
        );
        f.restart("2026-11-01");
        assert_eq!(f.reopen(0).tasks, initial.tasks);
        assert_eq!(f.complete(0, "2026-11-01").tasks[1].title, "Revue");
    }
}

#[test]
fn une_occurrence_ancienne_nest_plus_reouvrable_apres_realisation_de_sa_suivante() {
    // RG-F6-13/14 — Quand deux occurrences sont réalisées le même jour.
    let mut f = Fixture::new();
    f.create("Revue /r jour /p 2026-11-01", "2026-11-01");
    f.complete(0, "2026-11-01");
    f.complete(1, "2026-11-01");
    f.restart("2026-11-01");
    let before = f.repo.snapshot();
    let text = f.text();
    assert!(application::reopen_task(&mut f.repo, before.revision, 0).is_err());
    assert_eq!(f.text(), text);
    let snapshot = f.reopen(1);
    assert_eq!(snapshot.tasks.len(), 2);
    assert!(snapshot.tasks[0].completed);
    assert!(!snapshot.tasks[1].completed);
}

#[test]
fn passer_le_temps_et_redemarrer_ne_genere_rien_avant_cochage() {
    // RG-F6-02/05 — Étant donné plusieurs vendredis manqués.
    let mut f = Fixture::new();
    f.create("Revue /r vendredi /p 2026-11-06", "2026-11-01");
    let text = f.text();
    for _ in 0..3 {
        f.restart("2026-11-23");
        assert!(!f.repo.reload_at(date("2026-11-23")).unwrap());
        assert_eq!(f.text(), text);
    }
    let snapshot = f.complete(0, "2026-11-23");
    for _ in 0..3 {
        f.restart("2026-11-24");
        assert_eq!(f.repo.snapshot().tasks, snapshot.tasks);
    }
}

#[test]
fn un_echec_ou_conflit_de_cochage_ne_confirme_aucune_moitie_du_cycle() {
    // Recette F6 / architecture §18 — Étant donné une proposition optimiste complète.
    for conflict in [false, true] {
        let mut f = Fixture::new();
        f.create("Revue /r jour /p 2026-11-01", "2026-11-01");
        let path = f.repo.path().to_owned();
        let mut store = TaskStore::new(f.repo);
        let before = store.snapshot.clone();
        let job = store
            .begin_write(
                store.source_revision(),
                TaskCommand::Complete {
                    position: 0,
                    today: date("2026-11-01"),
                },
            )
            .unwrap();
        assert_eq!(store.snapshot.tasks.len(), 2);
        if conflict {
            fs::write(&path, "- [ ] Extérieure\n").unwrap();
        } else {
            fs::remove_file(&path).unwrap();
            fs::create_dir(&path).unwrap();
        }
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = runtime.block_on(job.perform());
        assert!(result.error.is_some());
        store.finish(&result);
        assert_eq!(store.snapshot.tasks.len(), 1);
        assert!(!store.snapshot.tasks[0].completed);
        if conflict {
            assert_eq!(fs::read_to_string(path).unwrap(), "- [ ] Extérieure\n");
        } else {
            assert_eq!(store.snapshot.tasks, before.tasks);
            assert_eq!(store.snapshot.revision, before.revision);
        }
    }
}

#[test]
fn un_cochage_externe_attend_une_sauvegarde_applicative_pour_generer_la_suivante() {
    // Décision externe F6 — Étant donné un cochage daté ou observé sans date.
    for dated in [false, true] {
        let mut f = Fixture::new();
        f.create("Revue /r vendredi /p 2026-11-06", "2026-11-01");
        let external = f.text().replace("- [ ]", "- [x]").replace(
            '\n',
            if dated {
                " @completed(2026-11-23)\n"
            } else {
                "\n"
            },
        );
        fs::write(f.repo.path(), &external).unwrap();
        f.repo.reload_at(date("2026-11-23")).unwrap();
        assert_eq!(f.repo.snapshot().tasks.len(), 1);
        assert_eq!(f.text(), external);
        let snapshot = f.create("Ponctuelle", "2026-11-23");
        assert_eq!(snapshot.tasks.len(), 3);
        assert_eq!(snapshot.tasks[2].planned, Some(date("2026-11-27")));
        assert_eq!(snapshot.tasks[0].completed_date, Some(date("2026-11-23")));
        f.restart("2026-11-24");
        f.create("Autre", "2026-11-24");
        assert_eq!(
            f.repo
                .snapshot()
                .tasks
                .iter()
                .filter(|t| t.recurrence.is_some() && !t.completed)
                .count(),
            1
        );
    }
}

#[test]
fn les_echecs_de_creation_et_edition_ne_laissent_pas_de_metadonnees_partielles() {
    // Recette F6 — Quand le fichier devient inaccessible pendant une création ou une édition.
    for create in [false, true] {
        let mut f = Fixture::new();
        f.create("Revue /r vendredi /p 2026-11-06 #perso", "2026-11-01");
        let before = f.repo.snapshot();
        fs::remove_file(f.repo.path()).unwrap();
        fs::create_dir(f.repo.path()).unwrap();
        let input = parse_input(
            "Modifiée /r lundi /p 2026-11-09 #mission",
            date("2026-11-01"),
        )
        .unwrap();
        let result = if create {
            application::create_task_from_input(&mut f.repo, &input)
        } else {
            application::update_task(&mut f.repo, before.revision, 0, &input)
        };
        assert!(result.is_err());
        assert_eq!(f.repo.snapshot().tasks, before.tasks);
        assert_eq!(f.repo.snapshot().revision, before.revision);
    }
}

#[test]
fn une_edition_exterieure_dupliquee_ou_invalide_conserve_le_dernier_etat_valide() {
    // RG-F6-02 / architecture §13 — Étant donné une série et une ancienne tâche sans métadonnées F6.
    let mut f = Fixture::new();
    fs::write(f.repo.path(), "- [ ] Ancienne #perso\n").unwrap();
    f.repo.reload_at(date("2026-11-01")).unwrap();
    f.create("Revue /r vendredi /p 2026-11-06", "2026-11-01");
    let before = f.repo.snapshot();
    let text = f.text();
    let series_line = text.lines().nth(1).unwrap();
    for external in [
        format!("{text}{series_line}\n"),
        text.replace("@repeat(vendredi;", "@repeat(inconnu;"),
    ] {
        fs::write(f.repo.path(), &external).unwrap();
        assert!(f.repo.reload_at(date("2026-11-02")).is_err());
        assert_eq!(f.repo.snapshot().tasks, before.tasks);
        assert_eq!(f.repo.snapshot().revision, before.revision);
        assert_eq!(f.text(), external);
    }
}
