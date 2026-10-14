//! Contrat observable des projections et intentions IPC Tauri.
use super::*;
use mystline::{
    application::propose_task_change, presentation::input::parse_input, repository::codec,
};
use serde_json::{Value, json};

fn day(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, day).unwrap()
}

fn snapshot(text: &str) -> Snapshot {
    Snapshot {
        revision: 7,
        tasks: codec::parse(text, day(9)).unwrap(),
    }
}

fn source() -> SourceRevision {
    serde_json::from_value(json!({"generation": 2, "revision": 7})).unwrap()
}

fn view(snapshot: &Snapshot, request: &ViewRequest, today: NaiveDate) -> Value {
    serde_json::to_value(project(snapshot, source(), false, request, today, 21)).unwrap()
}

#[test]
fn le_contrat_ipc_preserve_source_version_sections_exclusives_et_metadonnees() {
    // RG-F2-08/09, F3-03 — Étant donné un vendredi, des tâches classables et cochées.
    let snapshot = snapshot(
        "- [ ] Sans date #fond\n- [ ] Lundi @planned(2026-10-12)\n- [ ] Aujourd'hui @planned(2026-10-09)\n- [ ] Retard @planned(2026-10-09) @deadline(2026-10-08)\n- [x] Achevée @deadline(2026-10-08) @completed(2026-10-09)\n- [x] Ancienne @completed(2026-10-08)\n",
    );
    // Quand la projection traverse le contrat JSON réellement exposé à Vue.
    let view = view(&snapshot, &ViewRequest::default(), day(9));
    // Alors les positions se rapportent au fichier, chaque tâche apparaît une fois.
    assert_eq!(view["source"], json!({"generation": 2, "revision": 7}));
    assert_eq!(view["version"], 21);
    assert_eq!(view["today"], "2026-10-09");
    assert_eq!(view["nextWorkday"], "2026-10-12");
    assert_eq!(view["busy"], false);
    let rows = view["rows"].as_array().unwrap();
    assert_eq!(
        rows.iter()
            .map(|row| (
                row["position"].as_u64().unwrap(),
                row["section"].as_str().unwrap()
            ))
            .collect::<Vec<_>>(),
        [
            (3, "overdue"),
            (2, "today"),
            (1, "nextWorkday"),
            (0, "all"),
            (4, "all")
        ]
    );
    assert_eq!(rows[4]["deadlineTone"], "future");
    assert_eq!(rows[4]["completedDate"], "2026-10-09");
    assert_eq!(rows[3]["tags"], json!(["fond"]));
}

#[test]
fn changement_de_jour_et_historique_utilisent_date_effective_et_filtres() {
    // RG-F3-04/05/07 — Étant donné deux dates et une complétion externe non datée.
    let snapshot = snapshot(
        "- [x] Support #Mission @completed(2026-10-08)\n- [x] SUPPORT récent #mission @completed(2026-10-09)\n- [x] Support observé #MISSION\n- [ ] Support actif #mission\n- [x] Support exclu #tech @completed(2026-10-09)\n",
    );
    let request = ViewRequest {
        query: "supp".into(),
        tag: Some("mIsSiOn".into()),
        history: true,
    };
    // Quand le lendemain est projeté, puis l'historique avec recherche et tag cumulés.
    let daily = view(&snapshot, &ViewRequest::default(), day(10));
    let history = view(&snapshot, &request, day(10));
    // Alors seules les tâches actives restent au quotidien, tri stable à date égale.
    assert_eq!(daily["rows"].as_array().unwrap().len(), 1);
    assert_eq!(daily["rows"][0]["position"], 3);
    assert_eq!(
        history["rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["position"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        [1, 2, 0]
    );
    assert_eq!(history["rows"][1]["completedDate"], "2026-10-09");
}

#[test]
fn le_brouillon_ipc_peut_etre_valide_sans_reinterpreter_les_tokens_du_titre() {
    // RG-F2-02/05 — Étant donné des caractères littéraux et des métadonnées indépendantes.
    let snapshot = snapshot(
        "- [ ] Relire \\#mission et \\/p vendredi #fond @planned(2026-10-12) @deadline(2026-10-20) @title-escaped(1)\n",
    );
    let view = view(&snapshot, &ViewRequest::default(), day(9));
    // Quand Vue reprend editDraft et le transmet sans modification.
    let draft = view["rows"][0]["editDraft"].as_str().unwrap();
    let input = parse_input(draft, day(10)).unwrap();
    let command = update(0, draft, day(10)).unwrap();
    let proposed = propose_task_change(&snapshot, 7, &command).unwrap();
    // Alors une autre date courante ne décale pas les dates et les tokens restent titre.
    assert_eq!(input.title, snapshot.tasks[0].title);
    assert_eq!(proposed.tasks, snapshot.tasks);
    assert!(update(0, "Titre /d impossible", day(9)).is_err());
    assert!(update(0, "/p demain #fond", day(9)).is_err());
}

#[test]
fn les_intentions_de_lhistorique_ou_de_taches_masquees_sont_refusees() {
    // RG-F3-01/05/08 — Étant donné une tâche active, une achevée aujourd'hui et une ancienne.
    let snapshot = snapshot(
        "- [ ] Active #mission\n- [x] Achevée #mission @completed(2026-10-09)\n- [x] Ancienne @completed(2026-10-08)\n",
    );
    let daily = ViewRequest::default();
    let history = ViewRequest {
        history: true,
        ..ViewRequest::default()
    };
    // Quand une intention passe le contrat sans se fier à l'état du bouton Vue.
    assert!(matches!(
        toggle(&snapshot, &daily, 0, day(9)).unwrap(),
        TaskCommand::Complete { position: 0, .. }
    ));
    assert!(matches!(
        toggle(&snapshot, &daily, 1, day(9)).unwrap(),
        TaskCommand::Reopen { position: 1 }
    ));
    // Alors seuls les éléments du quotidien peuvent changer d'état.
    assert!(toggle(&snapshot, &daily, 2, day(9)).is_err());
    assert!(toggle(&snapshot, &daily, 99, day(9)).is_err());
    assert!(toggle(&snapshot, &history, 0, day(9)).is_err());
    assert!(
        toggle(
            &snapshot,
            &ViewRequest {
                tag: Some("tech".into()),
                ..daily
            },
            0,
            day(9)
        )
        .is_err()
    );
}

#[test]
fn le_deplacement_filtre_permute_les_voisins_actifs_de_la_meme_section() {
    // RG-F2-13, F3-03 — Étant donné une tâche masquée et une terminée entre deux visibles.
    let snapshot = snapshot(
        "- [ ] A #mission\n- [ ] Masquée #tech\n- [x] Accomplie #mission @completed(2026-10-09)\n- [ ] B #mission\n- [ ] Aujourd'hui #mission @planned(2026-10-09)\n",
    );
    let request = ViewRequest {
        tag: Some("mission".into()),
        ..ViewRequest::default()
    };
    // Quand B est déplacée vers le haut dans la vue filtrée.
    let command = reorder(&snapshot, &request, 3, false, day(9)).unwrap();
    let proposed = propose_task_change(&snapshot, 7, &command).unwrap();
    // Alors les positions masquées/terminées restent fixes, les sections ne sont pas franchies.
    assert_eq!(
        proposed
            .tasks
            .iter()
            .map(|task| task.title.as_str())
            .collect::<Vec<_>>(),
        ["B", "Masquée", "Accomplie", "A", "Aujourd'hui"]
    );
    assert!(reorder(&snapshot, &request, 0, false, day(9)).is_none());
    assert!(reorder(&snapshot, &request, 3, true, day(9)).is_none());
    assert!(reorder(&snapshot, &request, 2, true, day(9)).is_none());
    assert!(
        reorder(
            &snapshot,
            &ViewRequest {
                history: true,
                ..request
            },
            0,
            true,
            day(9)
        )
        .is_none()
    );
}
