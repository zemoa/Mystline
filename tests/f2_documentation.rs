//! Spécifications exécutables F2 : chaque test raconte un parcours utilisateur.
//! Exécution : cargo test --test f2_documentation
use chrono::NaiveDate;
use mystline::{
    application::{self, CreateTaskError, TaskInput},
    presentation::{
        input::parse_input,
        panel::{PanelState, Section},
    },
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
            repo: TaskRepository::open(path, false).unwrap(),
            panel: PanelState::new(date(9)),
            _directory: directory,
        }
    }
    fn text(&self) -> String {
        fs::read_to_string(self.repo.path()).unwrap()
    }
    fn reopened(&self) -> Snapshot {
        TaskRepository::open(self.repo.path().to_owned(), false)
            .unwrap()
            .snapshot()
    }
}

#[test]
fn un_titre_suffit_a_creer_une_tache() {
    // RG-F2-01 — Étant donné un fichier vide, sans organisation préalable.
    let mut fixture = Fixture::new("");
    // Quand l'utilisateur valide un simple titre.
    let input = parse_input("  Acheter  du pain  ", date(9)).unwrap();
    let snapshot = application::create_task_from_input(&mut fixture.repo, &input).unwrap();
    // Alors la tâche active est dans Toutes les tâches et survit au redémarrage.
    assert_eq!(fixture.panel.rows(&snapshot)[0].section, Section::All);
    assert_eq!(fixture.reopened().tasks, snapshot.tasks);
    assert_eq!(fixture.text(), "- [ ] Acheter  du pain\n");
}

#[test]
fn une_capture_avec_metadonnees_est_persistee_et_retrouvee() {
    // RG-F2-01/02/03/06 — Étant donné un jeudi et un fichier vide.
    let mut fixture = Fixture::new("");
    // Quand une capture renseigne intention, deadline et contexte.
    let input = parse_input("Préparer le support /p jeudi /d vendredi #mission", date(8)).unwrap();
    let snapshot = application::create_task_from_input(&mut fixture.repo, &input).unwrap();
    // Alors les commandes ne font pas partie du titre et les données sont persistées.
    assert_eq!(snapshot.tasks[0].title, "Préparer le support");
    assert_eq!(snapshot.tasks[0].planned, Some(date(8)));
    assert_eq!(snapshot.tasks[0].deadline, Some(date(9)));
    assert_eq!(snapshot.tasks[0].tags, ["mission"]);
    assert_eq!(
        fixture.text(),
        "- [ ] Préparer le support #mission @planned(2026-10-08) @deadline(2026-10-09)\n"
    );
    assert_eq!(fixture.reopened().tasks, snapshot.tasks);
}

#[test]
fn une_edition_sans_changement_preserve_la_tache_apres_reouverture() {
    // RG-F2-02/05 — Étant donné un titre contenant des commandes littérales et des antislashs.
    let mut fixture = Fixture::new("");
    let literal = r"Relire #mission /p vendredi \\serveur\dossier @deadline(2026-10-20)";
    let before = application::create_task_from_input(
        &mut fixture.repo,
        &TaskInput {
            title: literal.into(),
            planned: Some(date(9)),
            deadline: Some(date(20)),
            tags: vec!["Équipe".into()],
        },
    )
    .unwrap();
    // Quand la tâche est rouverte puis enregistrée onze jours après sa création.
    fixture.panel.begin_edit(&before, 0);
    let after = fixture
        .panel
        .save_edit(&mut fixture.repo, date(20))
        .unwrap();
    // Alors dates, titre et tags restent identiques dans le cache et le fichier.
    assert_eq!(after.tasks, before.tasks);
    assert_eq!(fixture.reopened().tasks, before.tasks);
}

#[test]
fn retirer_une_commande_retire_sa_metadonnee() {
    // RG-F2-05 — Étant donné une tâche planifiée avec une deadline et deux tags.
    for draft in [
        "Support /d 2026-10-20 #mission #documentation",
        "Support /p 2026-10-09 #mission #documentation",
        "Support /p 2026-10-09 /d 2026-10-20 #documentation",
        "Support",
    ] {
        let mut fixture = Fixture::new(
            "- [ ] Support #mission #documentation @planned(2026-10-09) @deadline(2026-10-20)\n",
        );
        fixture.panel.begin_edit(&fixture.repo.snapshot(), 0);
        // Quand une ou plusieurs commandes sont retirées de l'édition.
        fixture.panel.editor.as_mut().unwrap().draft = draft.into();
        let snapshot = fixture.panel.save_edit(&mut fixture.repo, date(9)).unwrap();
        let expected = parse_input(draft, date(9)).unwrap();
        // Alors ces valeurs sont retirées, et seules les autres sont conservées après réouverture.
        let task = &snapshot.tasks[0];
        assert_eq!(task.planned, expected.planned);
        assert_eq!(task.deadline, expected.deadline);
        assert_eq!(task.tags, expected.tags);
        assert_eq!(fixture.reopened().tasks, snapshot.tasks);
    }
}

#[test]
fn planifier_une_tache_de_fond_prepare_le_prochain_jour_ouvre() {
    // RG-F2-05/06/08/09 — Étant donné une tâche ordinaire #fond un vendredi.
    let mut fixture = Fixture::new("- [ ] Améliorer la documentation #fond\n");
    fixture.panel.begin_edit(&fixture.repo.snapshot(), 0);
    // Quand elle est planifiée pour lundi.
    fixture.panel.editor.as_mut().unwrap().draft =
        "Améliorer la documentation #fond /p lundi".into();
    let snapshot = fixture.panel.save_edit(&mut fixture.repo, date(9)).unwrap();
    // Alors elle figure une fois dans Prochain jour ouvré et conserve son tag.
    assert_eq!(fixture.panel.rows(&snapshot).len(), 1);
    assert_eq!(
        fixture.panel.rows(&snapshot)[0].section,
        Section::NextWorkday
    );
    assert_eq!(snapshot.tasks[0].tags, ["fond"]);
    assert_eq!(fixture.reopened().tasks, snapshot.tasks);
}

#[test]
fn une_edition_invalide_conserve_la_saisie_et_la_tache_anterieure() {
    // RG-F2-04 — Étant donné une tâche existante et des saisies invalides.
    for draft in [
        "Support /d",
        "Support /p 2026-02-29",
        "Support /p lundi /p mardi",
        "Support /d lundi /d mardi",
        "/p lundi #mission",
        "Support #mission!",
        " \t ",
    ] {
        let mut fixture = Fixture::new("- [ ] Originale #mission @planned(2026-10-09)\n");
        let before = fixture.repo.snapshot();
        let text = fixture.text();
        fixture.panel.begin_edit(&before, 0);
        fixture.panel.editor.as_mut().unwrap().draft = draft.into();
        // Quand Entrée soumet une édition refusée.
        assert!(
            fixture
                .panel
                .save_edit(&mut fixture.repo, date(9))
                .is_none()
        );
        // Alors l'édition reste ouverte, le texte exact est récupérable et aucune écriture n'a eu lieu.
        let editor = fixture.panel.editor.as_ref().unwrap();
        assert_eq!(editor.draft, draft);
        assert!(editor.error.as_ref().is_some_and(|error| !error.is_empty()));
        assert_eq!(fixture.repo.snapshot().revision, before.revision);
        assert_eq!(fixture.repo.snapshot().tasks, before.tasks);
        assert_eq!(fixture.text(), text);
    }
}

#[test]
fn echap_annule_ledition_sans_ecrire() {
    // RG-F2-05 — Étant donné une édition modifiée.
    let mut fixture = Fixture::new("- [ ] Originale\n");
    let before = fixture.repo.snapshot();
    let text = fixture.text();
    fixture.panel.begin_edit(&before, 0);
    fixture.panel.editor.as_mut().unwrap().draft = "Nouvelle /p lundi #mission".into();
    // Quand Échap annule.
    assert!(fixture.panel.escape());
    // Alors la tâche et le fichier ne changent pas.
    assert!(fixture.panel.editor.is_none());
    assert_eq!(fixture.repo.snapshot().tasks, before.tasks);
    assert_eq!(fixture.text(), text);
}

#[test]
fn le_reordonnancement_filtre_permute_les_positions_visibles() {
    // RG-F2-13 — Étant donné plusieurs lignes masquées entre A et B et une tâche d'une autre section.
    let mut fixture = Fixture::new(
        "- [ ] A #visible\n- [ ] Masquée 1\n- [x] Terminée @completed(2026-10-01)\n- [ ] Masquée 2\n- [ ] B #visible\n- [ ] Jour #visible @planned(2026-10-09)\n",
    );
    fixture.panel.tag = Some("visible".into());
    let before = fixture.repo.snapshot();
    let neighbor = fixture.panel.neighbor(&before, 4, false).unwrap();
    // Quand B est déplacée vers la position visible précédente.
    let after =
        application::reorder_tasks(&mut fixture.repo, before.revision, 4, neighbor).unwrap();
    // Alors seules A et B sont échangées ; les lignes masquées conservent leurs positions et leur ordre.
    assert_eq!(after.tasks[0].title, "B");
    assert_eq!(after.tasks[4].title, "A");
    for index in [1, 2, 3, 5] {
        assert_eq!(after.tasks[index], before.tasks[index]);
    }
    assert_eq!(fixture.reopened().tasks, after.tasks);
    // Et le déplacement inverse restitue l'ordre initial.
    let restored = application::reorder_tasks(&mut fixture.repo, after.revision, 0, 4).unwrap();
    assert_eq!(restored.tasks, before.tasks);
}

#[test]
fn recherche_filtre_et_changement_de_jour_necrivent_pas() {
    // RG-F2-09/11/12 — Étant donné une vue du vendredi.
    let mut fixture = Fixture::new(
        "- [ ] Support #mission @planned(2026-10-09)\n- [ ] Autre #mission @deadline(2026-10-09)\n",
    );
    let before = fixture.repo.snapshot();
    let text = fixture.text();
    // Quand on recherche, filtre puis passe au samedi.
    fixture.panel.query = "supp".into();
    fixture.panel.tag = Some("MISSION".into());
    assert_eq!(fixture.panel.rows(&before)[0].section, Section::Today);
    fixture.panel.today = date(10);
    assert_eq!(fixture.panel.rows(&before)[0].section, Section::All);
    fixture.panel.query.clear();
    assert_eq!(fixture.panel.rows(&before)[0].section, Section::Overdue);
    // Alors les tâches et le fichier demeurent inchangés.
    assert_eq!(fixture.text(), text);
    assert_eq!(fixture.repo.snapshot().revision, before.revision);
    assert_eq!(fixture.repo.snapshot().tasks, before.tasks);
}

#[test]
fn un_conflit_necrase_pas_une_edition_exterieure() {
    // F0 / RG-F2-05 — Étant donné une édition ouverte, avant notification du watcher.
    let mut fixture = Fixture::new("- [ ] Originale\n");
    fixture.panel.begin_edit(&fixture.repo.snapshot(), 0);
    fixture.panel.editor.as_mut().unwrap().draft = "Mon brouillon /p lundi".into();
    // Quand un éditeur externe remplace la liste avant Entrée.
    let external = "- [ ] Autre ligne\n- [ ] Originale modifiée extérieurement\n";
    fs::write(fixture.repo.path(), external).unwrap();
    assert!(
        fixture
            .panel
            .save_edit(&mut fixture.repo, date(9))
            .is_none()
    );
    // Alors le fichier extérieur et le brouillon sont préservés, sans viser la nouvelle ligne 0.
    let editor = fixture.panel.editor.as_ref().unwrap();
    assert_eq!(editor.draft, "Mon brouillon /p lundi");
    assert!(editor.invalidated);
    assert_eq!(fixture.text(), external);
    let revision = fixture.repo.snapshot().revision;
    assert!(
        fixture
            .panel
            .save_edit(&mut fixture.repo, date(9))
            .is_none()
    );
    assert_eq!(fixture.repo.snapshot().revision, revision);
    assert_eq!(fixture.text(), external);
}

#[test]
fn une_revision_obsolete_refuse_edition_et_permutation() {
    // F0 / RG-F2-05/13 — Étant donné un snapshot déjà remplacé par une édition externe.
    let mut fixture = Fixture::new("- [ ] A\n- [ ] B\n");
    let before = fixture.repo.snapshot();
    fs::write(
        fixture.repo.path(),
        "- [ ] B extérieure\n- [ ] A extérieure\n",
    )
    .unwrap();
    fixture.repo.reload().unwrap();
    let text = fixture.text();
    // Quand des commandes utilisent l'ancienne révision.
    for result in [
        application::update_task(
            &mut fixture.repo,
            before.revision,
            0,
            &TaskInput {
                title: "Mauvaise ligne".into(),
                ..TaskInput::default()
            },
        ),
        application::reorder_tasks(&mut fixture.repo, before.revision, 0, 1),
    ] {
        // Alors elles sont refusées et ne touchent pas au fichier.
        assert!(matches!(
            result,
            Err(CreateTaskError::Repository(RepositoryError::Stale))
        ));
    }
    assert_eq!(fixture.text(), text);
}

#[test]
fn un_conflit_de_permutation_preserve_lordre_exterieur() {
    // F0 / RG-F2-13 — Étant donné A et B sélectionnés depuis un snapshot courant.
    let mut fixture = Fixture::new("- [ ] A\n- [ ] B\n");
    let revision = fixture.repo.snapshot().revision;
    // Quand une modification extérieure survient juste avant le déplacement.
    let external = "- [ ] Extérieure\n- [ ] B\n- [ ] A\n";
    fs::write(fixture.repo.path(), external).unwrap();
    let result = application::reorder_tasks(&mut fixture.repo, revision, 0, 1);
    // Alors l'action est refusée et l'ordre extérieur devient le snapshot confirmé.
    assert!(matches!(
        result,
        Err(CreateTaskError::Repository(RepositoryError::Changed))
    ));
    assert_eq!(fixture.text(), external);
    assert_eq!(fixture.repo.snapshot().tasks[0].title, "Extérieure");
}

#[test]
fn un_echec_decriture_ne_confirme_pas_la_modification() {
    // F0 / RG-F2-04/05/13 — Étant donné un fichier devenu inaccessible (remplacé par un répertoire).
    let mut fixture = Fixture::new("- [ ] A\n- [ ] B\n");
    let before = fixture.repo.snapshot();
    fixture.panel.begin_edit(&before, 0);
    fixture.panel.editor.as_mut().unwrap().draft = "Brouillon conservé".into();
    fs::remove_file(fixture.repo.path()).unwrap();
    fs::create_dir(fixture.repo.path()).unwrap();
    // Quand l'édition et une permutation tentent d'écrire.
    assert!(
        fixture
            .panel
            .save_edit(&mut fixture.repo, date(9))
            .is_none()
    );
    assert!(application::reorder_tasks(&mut fixture.repo, before.revision, 0, 1).is_err());
    // Alors aucune modification n'est confirmée et le brouillon reste récupérable.
    assert_eq!(fixture.repo.snapshot().tasks, before.tasks);
    assert_eq!(fixture.repo.snapshot().revision, before.revision);
    assert_eq!(
        fixture.panel.editor.as_ref().unwrap().draft,
        "Brouillon conservé"
    );
    assert!(fixture.panel.editor.as_ref().unwrap().error.is_some());
}

#[test]
fn un_changement_de_source_invalide_ledition_meme_a_revision_egale() {
    // F0 / RG-F2-05 — Étant donné deux fichiers à la même révision runtime.
    let mut fixture = Fixture::new("- [ ] Première source\n");
    let mut other = Fixture::new("- [ ] Autre source\n");
    fixture.panel.begin_edit(&fixture.repo.snapshot(), 0);
    fixture.panel.editor.as_mut().unwrap().draft = "Brouillon première source".into();
    // Quand les paramètres changent la source de tâches.
    fixture.panel.invalidate_edit();
    let before = other.text();
    // Alors le brouillon ne peut pas être enregistré dans l'autre fichier.
    assert!(fixture.panel.save_edit(&mut other.repo, date(9)).is_none());
    assert_eq!(other.text(), before);
    assert_eq!(
        fixture.panel.editor.as_ref().unwrap().draft,
        "Brouillon première source"
    );
}

#[test]
fn les_tags_exterieurs_de_casses_differentes_designent_le_meme_tag() {
    // RG-F2-06 / F0 — Étant donné un fichier extérieur contenant des tags répétés.
    let fixture = Fixture::new("- [ ] Tâche #Mission #mission #Équipe #équipe\n");
    let original = fixture.text();
    // Quand il est chargé, alors les graphies sont dédupliquées dans le snapshot.
    assert_eq!(fixture.repo.snapshot().tasks[0].tags, ["Mission", "Équipe"]);
    // Et cette lecture ne normalise pas spontanément le fichier extérieur.
    assert_eq!(fixture.text(), original);
}
