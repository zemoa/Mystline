use super::input::{format_edit, parse_input};
use crate::{
    application,
    domain::{Task, same_tag},
    repository::{Snapshot, TaskRepository},
};
use chrono::{DateTime, Datelike, Days, NaiveDate, TimeZone};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Overdue,
    Today,
    NextWorkday,
    All,
}

impl Section {
    pub const ALL: [Self; 4] = [Self::Overdue, Self::Today, Self::NextWorkday, Self::All];
    pub fn label(self) -> &'static str {
        match self {
            Self::Overdue => "En retard",
            Self::Today => "Aujourd'hui",
            Self::NextWorkday => "Prochain jour ouvré",
            Self::All => "Toutes les tâches",
        }
    }
}

pub fn next_workday(today: NaiveDate) -> NaiveDate {
    let days = match today.weekday().num_days_from_monday() {
        4 => 3,
        5 => 2,
        _ => 1,
    };
    today
        .checked_add_days(Days::new(days))
        .expect("date locale représentable")
}

pub fn section(task: &Task, today: NaiveDate) -> Section {
    if !task.completed && task.deadline.is_some_and(|date| date < today) {
        Section::Overdue
    } else if task.planned == Some(today) {
        Section::Today
    } else if task.planned == Some(next_workday(today)) {
        Section::NextWorkday
    } else {
        Section::All
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeadlineTone {
    Overdue,
    Today,
    Future,
}

pub fn deadline_tone(date: NaiveDate, today: NaiveDate) -> DeadlineTone {
    if date < today {
        DeadlineTone::Overdue
    } else if date == today {
        DeadlineTone::Today
    } else {
        DeadlineTone::Future
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    pub index: usize,
    pub section: Section,
}

pub struct Editor {
    pub position: usize,
    pub revision: u64,
    pub draft: String,
    pub error: Option<String>,
    pub help: bool,
    pub invalidated: bool,
}

pub struct PanelState {
    pub today: NaiveDate,
    pub query: String,
    pub search_active: bool,
    pub tag: Option<String>,
    pub editor: Option<Editor>,
    pub error: Option<String>,
}

impl PanelState {
    pub fn new(today: NaiveDate) -> Self {
        Self {
            today,
            query: String::new(),
            search_active: false,
            tag: None,
            editor: None,
            error: None,
        }
    }

    pub fn rows(&self, snapshot: &Snapshot) -> Vec<Row> {
        let query = self.query.to_lowercase();
        let mut rows = Vec::new();
        for group in Section::ALL {
            rows.extend(
                snapshot
                    .tasks
                    .iter()
                    .enumerate()
                    .filter_map(|(index, task)| {
                        let matches_query = task.title.to_lowercase().contains(&query)
                            || task
                                .tags
                                .iter()
                                .any(|tag| tag.to_lowercase().contains(&query));
                        let matches_tag = self.tag.as_ref().is_none_or(|tag| {
                            task.tags.iter().any(|candidate| same_tag(candidate, tag))
                        });
                        (!task.completed
                            && section(task, self.today) == group
                            && matches_query
                            && matches_tag)
                            .then_some(Row {
                                index,
                                section: group,
                            })
                    }),
            );
        }
        rows
    }

    pub fn indices(&self, snapshot: &Snapshot) -> Vec<usize> {
        self.rows(snapshot).iter().map(|row| row.index).collect()
    }

    pub fn begin_edit(&mut self, snapshot: &Snapshot, position: usize) {
        if let Some(task) = snapshot.tasks.get(position).filter(|task| !task.completed) {
            self.editor = Some(Editor {
                position,
                revision: snapshot.revision,
                draft: format_edit(task),
                error: None,
                help: false,
                invalidated: false,
            });
            self.search_active = false;
        }
    }

    pub fn invalidate_edit(&mut self) {
        if let Some(editor) = &mut self.editor {
            editor.invalidated = true;
            editor.error = Some("La liste a changé. Conserver ce texte si nécessaire, puis Échap et rouvrir la tâche pour l'éditer.".into());
        }
    }

    pub fn refresh(&mut self, snapshot: &Snapshot) {
        if self
            .editor
            .as_ref()
            .is_some_and(|edit| edit.revision != snapshot.revision)
        {
            self.invalidate_edit();
        }
    }

    pub fn save_edit(&mut self, repo: &mut TaskRepository, today: NaiveDate) -> Option<Snapshot> {
        let editor = self.editor.as_mut()?;
        if editor.invalidated {
            return None;
        }
        let result = parse_input(&editor.draft, today)
            .map_err(|error| error.to_string())
            .and_then(|input| {
                application::update_task(repo, editor.revision, editor.position, &input)
                    .map_err(|error| error.to_string())
            });
        match result {
            Ok(snapshot) => {
                self.editor = None;
                self.error = None;
                Some(snapshot)
            }
            Err(error) => {
                editor.error = Some(error);
                self.refresh(&repo.snapshot());
                None
            }
        }
    }

    /// Renvoie le voisin visible de la même section ; aux bornes, aucune action.
    pub fn neighbor(&self, snapshot: &Snapshot, selected: usize, down: bool) -> Option<usize> {
        let rows = self.rows(snapshot);
        let at = rows.iter().position(|row| row.index == selected)?;
        let target = if down {
            at.checked_add(1)?
        } else {
            at.checked_sub(1)?
        };
        rows.get(target)
            .filter(|row| row.section == rows[at].section)
            .map(|row| row.index)
    }

    /// Échap consomme une interaction interne avant de fermer le panneau.
    pub fn escape(&mut self) -> bool {
        if let Some(editor) = &mut self.editor {
            if editor.help {
                editor.help = false;
            } else {
                self.editor = None;
            }
            return true;
        }
        if self.search_active || !self.query.is_empty() {
            self.search_active = false;
            self.query.clear();
            return true;
        }
        false
    }
}

/// Un seul réveil au prochain début de journée locale, même si minuit est
/// ambigu ou inexistant lors d'un changement de fuseau / d'heure d'été.
pub fn next_day_delay<Tz: TimeZone>(now: DateTime<Tz>) -> Duration {
    let next = now
        .date_naive()
        .succ_opt()
        .expect("date locale représentable");
    let mut candidate = next.and_hms_opt(0, 0, 0).unwrap();
    loop {
        if let Some(deadline) = now.timezone().from_local_datetime(&candidate).earliest() {
            return (deadline - now).to_std().unwrap_or(Duration::from_secs(1));
        }
        candidate += chrono::Duration::minutes(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::codec;
    fn date(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, day).unwrap()
    }
    fn snapshot(text: &str) -> Snapshot {
        Snapshot {
            revision: 3,
            tasks: codec::parse(text, date(9)).unwrap(),
        }
    }

    #[test]
    fn les_sections_sont_exclusives_et_ordonnees() {
        // RG-F2-08/09 — Étant donné un vendredi et un fichier volontairement désordonné.
        let snapshot = snapshot(
            "- [ ] Sans date\n- [ ] Lundi @planned(2026-10-12)\n- [ ] Hier @planned(2026-10-08)\n- [ ] Aujourd'hui @planned(2026-10-09)\n- [ ] En retard @planned(2026-10-09) @deadline(2026-10-08)\n- [ ] Plus tard @planned(2026-10-20)\n- [x] Terminée @deadline(2026-10-01)\n",
        );
        // Quand le panneau calcule ses lignes.
        let rows = PanelState::new(date(9)).rows(&snapshot);
        // Alors le retard prime, les anciennes planifications restent visibles et aucun doublon n'existe.
        assert_eq!(
            rows,
            [
                Row {
                    index: 4,
                    section: Section::Overdue
                },
                Row {
                    index: 3,
                    section: Section::Today
                },
                Row {
                    index: 1,
                    section: Section::NextWorkday
                },
                Row {
                    index: 0,
                    section: Section::All
                },
                Row {
                    index: 2,
                    section: Section::All
                },
                Row {
                    index: 5,
                    section: Section::All
                }
            ]
        );
    }

    #[test]
    fn le_prochain_jour_ouvre_ignore_le_weekend() {
        // RG-F2-09 — Étant donné chacun des jours d'une semaine.
        for (today, next) in [(5, 6), (6, 7), (7, 8), (8, 9), (9, 12), (10, 12), (11, 12)] {
            // Quand on calcule le prochain jour ouvré, alors il est strictement futur et lundi-vendredi.
            assert_eq!(next_workday(date(today)), date(next));
        }
    }

    #[test]
    fn une_deadline_future_ne_planifie_pas_la_tache() {
        // RG-F2-10 — Étant donné des deadlines aujourd'hui et éloignées, sans planification.
        let snapshot =
            snapshot("- [ ] Urgente @deadline(2026-10-09)\n- [ ] Éloignée @deadline(2026-10-25)\n");
        // Quand le panneau les présente, alors elles restent dans Toutes les tâches.
        assert!(
            PanelState::new(date(9))
                .rows(&snapshot)
                .iter()
                .all(|row| row.section == Section::All)
        );
        assert_eq!(deadline_tone(date(9), date(9)), DeadlineTone::Today);
        assert_eq!(deadline_tone(date(25), date(9)), DeadlineTone::Future);
        assert_eq!(deadline_tone(date(8), date(9)), DeadlineTone::Overdue);
    }

    #[test]
    fn recherche_et_filtre_par_tag_se_croisent_sans_changer_les_sections() {
        // RG-F2-11/12 — Étant donné des titres et tags de casses différentes.
        let snapshot = snapshot(
            "- [ ] Préparer SUPPORT #Mission @planned(2026-10-09)\n- [ ] support technique #tech\n- [ ] Relire #SUPPORT #mission @planned(2026-10-12)\n",
        );
        let before = snapshot.tasks.clone();
        let mut panel = PanelState::new(date(9));
        // Quand on recherche une partie du titre ou du tag et applique un tag exact.
        panel.query = "supp".into();
        assert_eq!(panel.indices(&snapshot), [0, 2, 1]);
        panel.tag = Some("MISSION".into());
        assert_eq!(panel.indices(&snapshot), [0, 2]);
        assert_eq!(panel.rows(&snapshot)[1].section, Section::NextWorkday);
        // Alors retirer le filtre restaure la recherche, et aucune tâche n'est modifiée.
        panel.tag = None;
        assert_eq!(panel.indices(&snapshot), [0, 2, 1]);
        panel.query = "introuvable".into();
        assert!(panel.indices(&snapshot).is_empty());
        assert_eq!(snapshot.tasks, before);
    }

    #[test]
    fn le_reordonnancement_respecte_les_sections_et_les_bornes() {
        // RG-F2-13 — Étant donné deux tâches visibles séparées par une ligne masquée et une autre section.
        let snapshot = snapshot(
            "- [ ] A #visible\n- [ ] Masquée\n- [ ] B #visible\n- [ ] Jour #visible @planned(2026-10-09)\n",
        );
        let mut panel = PanelState::new(date(9));
        panel.tag = Some("visible".into());
        // Quand on demande les voisins, alors seules les lignes visibles de la même section sont proposées.
        assert_eq!(panel.neighbor(&snapshot, 2, false), Some(0));
        assert_eq!(panel.neighbor(&snapshot, 0, true), Some(2));
        assert_eq!(panel.neighbor(&snapshot, 0, false), None);
        assert_eq!(panel.neighbor(&snapshot, 2, true), None);
        assert_eq!(panel.neighbor(&snapshot, 3, true), None);
    }

    #[test]
    fn le_changement_de_jour_reclasse_sans_modifier_les_taches() {
        // RG-F2-09 — Étant donné un panneau ouvert le vendredi.
        let snapshot = snapshot(
            "- [ ] Vendredi @planned(2026-10-09)\n- [ ] Lundi @planned(2026-10-12)\n- [ ] Deadline @deadline(2026-10-09)\n",
        );
        let before = snapshot.tasks.clone();
        let mut panel = PanelState::new(date(9));
        assert_eq!(panel.indices(&snapshot), [0, 1, 2]);
        // Quand la date passe à samedi, alors les sections changent sans report automatique.
        panel.today = date(10);
        assert_eq!(panel.indices(&snapshot), [2, 1, 0]);
        assert_eq!(snapshot.tasks, before);
    }

    #[test]
    fn le_minuteur_vise_le_prochain_minuit_local() {
        // RG-F2-09 — Étant donné un fuseau +02:00 et deux heures locales.
        let zone = chrono::FixedOffset::east_opt(7200).unwrap();
        for (hour, minute, second, expected) in [(23, 59, 50, 10), (12, 0, 0, 43200)] {
            // Quand le délai est calculé, alors il vise exactement le lendemain local.
            let now = zone
                .with_ymd_and_hms(2026, 10, 9, hour, minute, second)
                .unwrap();
            assert_eq!(next_day_delay(now), Duration::from_secs(expected));
        }
    }

    #[test]
    fn echap_ferme_laide_puis_ledition_puis_la_recherche() {
        // RG-F2-05/07/14 — Étant donné une édition avec aide et une recherche persistante.
        let snapshot = snapshot("- [ ] Originale\n");
        let mut panel = PanelState::new(date(9));
        panel.query = "orig".into();
        panel.begin_edit(&snapshot, 0);
        let editor = panel.editor.as_mut().unwrap();
        editor.draft = "Modifiée".into();
        editor.help = true;
        // Quand Échap est pressé successivement, alors chaque niveau se ferme sans modifier la tâche.
        assert!(panel.escape());
        assert!(!panel.editor.as_ref().unwrap().help);
        assert_eq!(panel.editor.as_ref().unwrap().draft, "Modifiée");
        assert!(panel.escape());
        assert!(panel.editor.is_none());
        assert!(panel.escape());
        assert!(panel.query.is_empty());
        assert!(!panel.escape());
        assert_eq!(snapshot.tasks[0].title, "Originale");
    }
}
