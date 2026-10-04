//! Spécifications exécutables F3 : scénarios utilisateur, dates fixes et fichier réel.
//! Exécution : cargo test --test f3_documentation
use chrono::NaiveDate;
use mystline::{
    application::{self, CreateTaskError},
    presentation::panel::{DeadlineTone, PanelMode, PanelState, Section, task_deadline_tone},
    repository::{RepositoryError, Snapshot, TaskRepository},
};
use std::fs;

fn date(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, day).unwrap()
}

struct Fixture {
    _directory: tempfile::TempDir,
    repo: TaskRepository,
    panel: PanelState,
}

impl Fixture {
    fn new(markdown: &str) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("tasks.md");
        fs::write(&path, markdown).unwrap();
        Self {
            repo: TaskRepository::open_at(path, false, date(9)).unwrap(),
            panel: PanelState::new(date(9)),
            _directory: directory,
        }
    }
    fn text(&self) -> String {
        fs::read_to_string(self.repo.path()).unwrap()
    }
    fn reopened(&self, today: NaiveDate) -> Snapshot {
        TaskRepository::open_at(self.repo.path().to_owned(), false, today)
            .unwrap()
            .snapshot()
    }
    fn complete(&mut self, position: usize) -> Snapshot {
        let revision = self.repo.snapshot().revision;
        self.panel
            .complete(&mut self.repo, revision, position, date(9))
            .unwrap()
    }
}

#[test]
fn terminer_une_tache_conserve_ses_metadonnees_et_persiste_sa_date() {
    // RG-F3-01/02 — Étant donné une tâche avec planification, deadline et tags.
    let mut f = Fixture::new(
        "- [ ] Support #mission #documentation @planned(2026-10-09) @deadline(2026-10-08)\n",
    );
    let original = f.repo.snapshot().tasks.remove(0);
    // Quand la tâche est cochée dans l'application.
    let snapshot = f.complete(0);
    let task = &snapshot.tasks[0];
    // Alors son état et sa date sont persistés, toutes ses métadonnées sont conservées.
    assert!(task.completed);
    assert_eq!(task.completed_date, Some(date(9)));
    assert_eq!(task.observed_completion, None);
    assert_eq!(task.title, original.title);
    assert_eq!(task.planned, original.planned);
    assert_eq!(task.deadline, original.deadline);
    assert_eq!(task.tags, original.tags);
    assert_eq!(
        f.text(),
        "- [x] Support #mission #documentation @planned(2026-10-09) @deadline(2026-10-08) @completed(2026-10-09)\n"
    );
    assert_eq!(f.reopened(date(10)).tasks, snapshot.tasks);
}

#[test]
fn une_tache_terminee_aujourdhui_reste_dans_une_seule_section() {
    // RG-F3-03 — Étant donné un vendredi et toutes les possibilités de planification.
    for (planned, expected) in [
        (" @planned(2026-10-09)", Section::Today),
        (" @planned(2026-10-12)", Section::NextWorkday),
        (" @planned(2026-10-20)", Section::All),
        (" @planned(2026-10-08)", Section::All),
        ("", Section::All),
    ] {
        let mut f = Fixture::new(&format!("- [ ] Support{planned} @deadline(2026-10-08)\n"));
        // Quand la tâche en retard est terminée.
        let snapshot = f.complete(0);
        let rows = f.panel.rows(&snapshot);
        // Alors elle reste visible une seule fois et sa planification décide de la section.
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].section, expected);
        assert!(snapshot.tasks[rows[0].index].completed);
    }
}

#[test]
fn terminer_une_tache_en_retard_la_reclasse_sans_signaler_sa_deadline() {
    // RG-F3-03 — Étant donné une tâche sans planification, avec une deadline dépassée.
    let mut f = Fixture::new("- [ ] Contrat @deadline(2026-10-08)\n");
    assert_eq!(
        f.panel.rows(&f.repo.snapshot())[0].section,
        Section::Overdue
    );
    // Quand elle est cochée.
    let snapshot = f.complete(0);
    // Alors elle quitte En retard et reste terminée dans Toutes les tâches.
    assert_eq!(f.panel.rows(&snapshot)[0].section, Section::All);
    assert_eq!(snapshot.tasks[0].deadline, Some(date(8)));
    assert_eq!(
        task_deadline_tone(&snapshot.tasks[0], date(9)),
        Some(DeadlineTone::Future)
    );
}

#[test]
fn le_lendemain_retire_la_tache_du_panneau_et_la_conserve_dans_lhistorique() {
    // RG-F3-04/05 — Étant donné une tâche terminée vendredi, dans un panneau ouvert.
    let mut f = Fixture::new("- [ ] Support @planned(2026-10-09)\n");
    let snapshot = f.complete(0);
    let text = f.text();
    assert_eq!(f.panel.indices(&snapshot), [0]);
    // Quand la date locale passe au samedi, sans fermer le panneau.
    f.panel.today = date(10);
    // Alors la tâche disparaît, sans écriture ; après redémarrage elle reste dans l'historique.
    assert!(f.panel.rows(&snapshot).is_empty());
    assert_eq!(f.text(), text);
    assert_eq!(f.repo.snapshot().revision, snapshot.revision);
    let reopened = f.reopened(date(10));
    assert!(f.panel.rows(&reopened).is_empty());
    f.panel.toggle_history();
    assert_eq!(f.panel.indices(&reopened), [0]);
    assert_eq!(reopened.tasks[0].completed_date, Some(date(9)));
}

#[test]
fn lhistorique_inclut_aujourdhui_et_se_filtre_du_plus_recent_au_plus_ancien() {
    // RG-F3-05 — Étant donné des dates et des lignes volontairement désordonnées.
    let mut f = Fixture::new(
        "- [x] Support ancien #Mission @completed(2026-10-07)\n- [ ] Support actif #mission\n- [x] Support récent A #mission @completed(2026-10-09)\n- [x] Autre #tech @completed(2026-10-08)\n- [x] Support récent B #mission @completed(2026-10-09)\n",
    );
    let before = f.repo.snapshot();
    let text = f.text();
    // Quand l'historique est ouvert, puis recherché et filtré.
    f.panel.toggle_history();
    assert_eq!(f.panel.indices(&before), [2, 4, 3, 0]);
    f.panel.query = "SUPP".into();
    f.panel.tag = Some("MISSION".into());
    // Alors aujourd'hui est inclus, l'ordre à date égale est stable et les filtres se combinent.
    assert_eq!(f.panel.indices(&before), [2, 4, 0]);
    f.panel.toggle_history();
    assert_eq!(f.panel.indices(&before), [1, 2, 4]);
    assert_eq!(f.panel.query, "SUPP");
    assert_eq!(f.panel.tag.as_deref(), Some("MISSION"));
    assert_eq!(f.text(), text);
    assert_eq!(f.repo.snapshot().revision, before.revision);
}

#[test]
fn une_completion_exterieure_datee_utilise_la_date_du_fichier() {
    // RG-F3-06 — Étant donné des tâches actives et un panneau courant.
    let mut f = Fixture::new("- [ ] Ancienne\n- [ ] Jour\n- [ ] Future\n");
    let external = "- [x] Ancienne @completed(2026-10-08)\n- [x] Jour @completed(2026-10-09)\n- [x] Future @completed(2026-10-10)\n";
    // Quand un éditeur externe coche les lignes avec des dates explicites.
    fs::write(f.repo.path(), external).unwrap();
    assert!(f.repo.reload_at(date(9)).unwrap());
    let snapshot = f.repo.snapshot();
    // Alors seule celle datée d'aujourd'hui est dans le panneau ; toutes sont dans l'historique.
    assert_eq!(f.panel.indices(&snapshot), [1]);
    f.panel.toggle_history();
    assert_eq!(f.panel.indices(&snapshot), [2, 1, 0]);
    assert_eq!(f.text(), external);
}

#[test]
fn une_completion_sans_date_attend_la_prochaine_ecriture_pour_etre_datee() {
    // RG-F3-07 — Étant donné une tâche active, cochée extérieurement vendredi sans date.
    let mut f = Fixture::new("- [ ] Sans date\n");
    fs::write(f.repo.path(), "- [x] Sans date\n").unwrap();
    // Quand le watcher recharge le fichier, puis le jour change.
    f.repo.reload_at(date(9)).unwrap();
    let observed = f.repo.snapshot();
    assert_eq!(observed.tasks[0].completed_date, None);
    assert_eq!(observed.tasks[0].observed_completion, Some(date(9)));
    assert_eq!(f.panel.indices(&observed), [0]);
    assert!(!f.repo.reload_at(date(10)).unwrap());
    assert_eq!(f.repo.snapshot().tasks, observed.tasks);
    f.panel.today = date(10);
    assert!(f.panel.indices(&observed).is_empty());
    assert_eq!(f.text(), "- [x] Sans date\n");
    // Quand une capture déclenche ensuite une écriture applicative.
    let confirmed = application::create_task(&mut f.repo, "Nouvelle").unwrap();
    // Alors la date de vendredi est fixée, dans le fichier et dans le cache.
    assert_eq!(confirmed.tasks[0].completed_date, Some(date(9)));
    assert_eq!(confirmed.tasks[0].observed_completion, None);
    assert_eq!(
        f.text(),
        "- [x] Sans date @completed(2026-10-09)\n- [ ] Nouvelle\n"
    );
    assert_eq!(f.reopened(date(10)).tasks, confirmed.tasks);
}

#[test]
fn un_rechargement_conserve_la_date_observee_apres_deplacement_de_la_ligne() {
    // RG-F3-07 — Étant donné une ligne sans date observée vendredi.
    let mut f = Fixture::new("- [x] Accomplie #mission\n- [ ] Autre\n");
    let external = "- [ ] Nouvelle\n- [ ] Autre modifiée\n- [x] Accomplie #mission\n";
    // Quand un éditeur insère et déplace des lignes samedi.
    fs::write(f.repo.path(), external).unwrap();
    f.repo.reload_at(date(10)).unwrap();
    f.panel.today = date(10);
    let snapshot = f.repo.snapshot();
    // Alors la date de vendredi reste effective et cette tâche ne réapparaît pas aujourd'hui.
    assert_eq!(snapshot.tasks[2].effective_completion_date(), Some(date(9)));
    assert_eq!(f.panel.indices(&snapshot), [0, 1]);
    assert_eq!(f.text(), external);
}

#[test]
fn une_correspondance_ambigue_ou_modifiee_est_observee_a_nouveau() {
    // RG-F3-07 — Étant donné des lignes sans identifiant persistant.
    for (original, external) in [
        ("- [x] Même\n", "- [x] Même\n- [x] Même\n"),
        ("- [x] Même\n- [x] Même\n", "- [x] Même\n"),
        ("- [x] Même #ancien\n", "- [x] Même #nouveau\n"),
    ] {
        let mut f = Fixture::new(original);
        // Quand une édition extérieure rend l'identité ambiguë ou change son contenu samedi.
        fs::write(f.repo.path(), external).unwrap();
        f.repo.reload_at(date(10)).unwrap();
        // Alors la nouvelle observation est utilisée, sans réécriture extérieure.
        assert!(
            f.repo
                .snapshot()
                .tasks
                .iter()
                .all(|task| task.effective_completion_date() == Some(date(10)))
        );
        assert_eq!(f.text(), external);
    }
}

#[test]
fn un_redemarrage_avant_ecriture_renouvelle_la_date_dobservation() {
    // RG-F3-07 — Étant donné une ligne sans date observée vendredi et jamais réécrite.
    let f = Fixture::new("- [x] Accomplie\n");
    assert_eq!(
        f.repo.snapshot().tasks[0].effective_completion_date(),
        Some(date(9))
    );
    // Quand une nouvelle session lit le même fichier samedi.
    let reopened = f.reopened(date(10));
    let mut panel = PanelState::new(date(10));
    // Alors l'exception MVP la rend visible samedi, puis la masque dimanche.
    assert_eq!(
        reopened.tasks[0].effective_completion_date(),
        Some(date(10))
    );
    assert_eq!(panel.indices(&reopened), [0]);
    panel.today = date(11);
    assert!(panel.indices(&reopened).is_empty());
    assert_eq!(f.text(), "- [x] Accomplie\n");
}

#[test]
fn un_echec_de_completion_preserve_letat_confirme_et_signale_lerreur() {
    // RG-F3-02 — Étant donné un fichier devenu inaccessible, remplacé par un répertoire.
    let mut f = Fixture::new("- [x] Déjà sans date\n- [ ] Support\n");
    let before = f.repo.snapshot();
    fs::remove_file(f.repo.path()).unwrap();
    fs::create_dir(f.repo.path()).unwrap();
    // Quand la complétion tente de sauvegarder.
    assert!(
        f.panel
            .complete(&mut f.repo, before.revision, 1, date(9))
            .is_none()
    );
    // Alors le panneau reste sur l'état confirmé et affiche une erreur.
    assert_eq!(f.repo.snapshot().tasks, before.tasks);
    assert_eq!(f.repo.snapshot().revision, before.revision);
    assert!(!f.repo.snapshot().tasks[1].completed);
    assert_eq!(f.repo.snapshot().tasks[0].completed_date, None);
    assert_eq!(
        f.repo.snapshot().tasks[0].observed_completion,
        Some(date(9))
    );
    assert!(
        f.panel
            .error
            .as_ref()
            .is_some_and(|error| !error.is_empty())
    );
    assert_eq!(f.panel.indices(&before), [0, 1]);
}

#[test]
fn un_conflit_de_completion_preserve_ledition_exterieure() {
    // RG-F3-02 / F0 — Étant donné une tâche sélectionnée avant notification du watcher.
    let mut f = Fixture::new("- [ ] Originale\n");
    let before = f.repo.snapshot();
    let external = "- [ ] Autre ligne\n- [ ] Originale extérieure\n";
    // Quand le fichier extérieur change avant le cochage de l'ancienne position.
    fs::write(f.repo.path(), external).unwrap();
    assert!(
        f.panel
            .complete(&mut f.repo, before.revision, 0, date(9))
            .is_none()
    );
    // Alors le snapshot extérieur est rechargé, aucune mauvaise ligne n'est cochée.
    assert_eq!(f.text(), external);
    assert_eq!(f.repo.snapshot().tasks[0].title, "Autre ligne");
    assert!(f.repo.snapshot().tasks.iter().all(|task| !task.completed));
    assert!(f.panel.error.is_some());
    // Et l'ancienne révision est désormais refusée sans toucher au fichier.
    assert!(matches!(
        application::complete_task(&mut f.repo, before.revision, 0, date(9)),
        Err(CreateTaskError::Repository(RepositoryError::Stale))
    ));
    assert_eq!(f.text(), external);
}

#[test]
fn une_tache_terminee_ne_peut_pas_etre_reouverte() {
    // RG-F3-01 — Étant donné une tâche déjà cochée.
    let mut f = Fixture::new("- [x] Accomplie @completed(2026-10-09)\n");
    let before = f.repo.snapshot();
    let text = f.text();
    // Quand une complétion répétée ou une position inexistante est demandée.
    for position in [0, 99] {
        assert!(matches!(
            application::complete_task(&mut f.repo, before.revision, position, date(10)),
            Err(CreateTaskError::InvalidPosition)
        ));
    }
    // Alors ni l'état, ni la date, ni la révision, ni le fichier ne changent.
    assert_eq!(f.repo.snapshot().tasks, before.tasks);
    assert_eq!(f.repo.snapshot().revision, before.revision);
    assert_eq!(f.text(), text);
    f.panel.begin_edit(&before, 0);
    assert!(f.panel.editor.is_none());
    assert_eq!(f.panel.neighbor(&before, 0, true), None);
}

#[test]
fn lhistorique_se_consulte_sans_modifier_les_taches() {
    // RG-F3-05/08 — Étant donné des tâches terminées et une recherche active.
    let mut f = Fixture::new("- [x] Support #mission @completed(2026-10-09)\n");
    let before = f.repo.snapshot();
    let text = f.text();
    f.panel.query = "supp".into();
    // Quand on ouvre l'historique puis presse Échap successivement.
    f.panel.toggle_history();
    f.panel.begin_edit(&before, 0);
    assert!(f.panel.editor.is_none());
    assert_eq!(f.panel.neighbor(&before, 0, true), None);
    assert!(
        f.panel
            .complete(&mut f.repo, before.revision, 0, date(9))
            .is_none()
    );
    assert!(f.panel.escape());
    assert_eq!(f.panel.mode, PanelMode::History);
    assert!(f.panel.query.is_empty());
    assert!(f.panel.escape());
    // Alors Échap retire d'abord la recherche, puis revient au quotidien avant de fermer.
    assert_eq!(f.panel.mode, PanelMode::Daily);
    assert!(!f.panel.escape());
    assert_eq!(f.text(), text);
    assert_eq!(f.repo.snapshot().revision, before.revision);
}

#[test]
fn le_reordonnancement_des_taches_actives_ignore_les_terminees_du_jour() {
    // RG-F3-03 / F2 — Étant donné une terminée du jour entre deux tâches actives.
    let mut f = Fixture::new("- [ ] A\n- [x] Accomplie @completed(2026-10-09)\n- [ ] B\n");
    let before = f.repo.snapshot();
    // Quand A est déplacée vers sa prochaine voisine active.
    let to = f.panel.neighbor(&before, 0, true).unwrap();
    assert_eq!(to, 2);
    let after = application::reorder_tasks(&mut f.repo, before.revision, 0, to).unwrap();
    // Alors seules A et B permutent, la tâche terminée garde sa position physique.
    assert_eq!(after.tasks[0].title, "B");
    assert_eq!(after.tasks[2].title, "A");
    assert_eq!(after.tasks[1], before.tasks[1]);
    assert_eq!(f.reopened(date(9)).tasks, after.tasks);
}
