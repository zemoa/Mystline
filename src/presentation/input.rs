use crate::{
    application::TaskInput,
    domain::{Task, same_tag, valid_tag},
};
use chrono::{Datelike, Days, NaiveDate};
use thiserror::Error;

pub const HELP: &str = " /p <date> — prévoir une tâche\n /d <date> — définir une deadline\n #tag — ajouter du contexte (plusieurs tags possibles)\n Dates : YYYY-MM-DD, aujourd'hui, demain, lundi à dimanche\n \\#tag et \\/p — conserver ces tokens dans le titre\n \\\\ — écrire un antislash littéral\n F1 : afficher / fermer l'aide · Échap : fermer d'abord l'aide";

#[derive(Debug, Error, PartialEq, Eq)]
pub enum InputError {
    #[error("Saisir un titre, en plus des commandes éventuelles")]
    EmptyTitle,
    #[error(
        "{0} : indiquer une date valide (YYYY-MM-DD, aujourd'hui, demain ou un jour de semaine)"
    )]
    InvalidDate(String),
    #[error("La commande {0} apparaît plusieurs fois : conserver une seule valeur")]
    DuplicateDate(String),
    #[error("Tag invalide : {0} (lettres, chiffres, - et _ ; sans espace)")]
    InvalidTag(String),
}

/// La date est fournie au moment de la validation, jamais à l'ouverture du champ.
pub fn parse_input(raw: &str, today: NaiveDate) -> Result<TaskInput, InputError> {
    let mut input = TaskInput::default();
    let mut tokens = raw.split_whitespace();
    let mut offset = 0;
    let mut previous_end = None;
    while let Some(token) = tokens.next() {
        let start = offset + raw[offset..].find(token).expect("token dans la saisie");
        offset = start + token.len();
        match token {
            "/p" | "/d" => {
                let slot = if token == "/p" {
                    &mut input.planned
                } else {
                    &mut input.deadline
                };
                if slot.is_some() {
                    return Err(InputError::DuplicateDate(token.into()));
                }
                let value = tokens
                    .next()
                    .ok_or_else(|| InputError::InvalidDate(token.into()))?;
                let value_start = offset + raw[offset..].find(value).expect("date dans la saisie");
                offset = value_start + value.len();
                *slot = Some(
                    parse_date(value, today)
                        .ok_or_else(|| InputError::InvalidDate(token.into()))?,
                );
                previous_end = None;
            }
            _ if token.starts_with('#') => {
                let tag = &token[1..];
                if !valid_tag(tag) {
                    return Err(InputError::InvalidTag(token.into()));
                }
                if !input.tags.iter().any(|old| same_tag(old, tag)) {
                    input.tags.push(tag.to_owned());
                }
                previous_end = None;
            }
            _ => {
                if !input.title.is_empty() {
                    input
                        .title
                        .push_str(previous_end.map_or(" ", |end| &raw[end..start]));
                }
                input.title.push_str(&decode_title(token));
                previous_end = Some(offset);
            }
        }
    }
    if input.title.is_empty() {
        return Err(InputError::EmptyTitle);
    }
    Ok(input)
}

fn parse_date(value: &str, today: NaiveDate) -> Option<NaiveDate> {
    let lowered = value.to_lowercase();
    match lowered.as_str() {
        "aujourd'hui" => Some(today),
        "demain" => today.checked_add_days(Days::new(1)),
        _ => {
            let days = [
                "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche",
            ];
            if let Some(day) = days.iter().position(|&day| day == lowered) {
                let distance = (day as u64 + 7 - today.weekday().num_days_from_monday() as u64) % 7;
                return today.checked_add_days(Days::new(distance));
            }
            let bytes = value.as_bytes();
            if bytes.len() != 10
                || bytes[4] != b'-'
                || bytes[7] != b'-'
                || !bytes
                    .iter()
                    .enumerate()
                    .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit())
            {
                return None;
            }
            NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
        }
    }
}

fn decode_title(token: &str) -> String {
    let mut chars = token.chars().peekable();
    let mut result = String::new();
    // Only a leading backslash escapes a command token; doubled backslashes
    // encode literal backslashes anywhere (including in Windows paths).
    if token.starts_with("\\#") || token == "\\/p" || token == "\\/d" {
        chars.next();
    }
    while let Some(c) = chars.next() {
        result.push(c);
        if c == '\\' && chars.peek() == Some(&'\\') {
            chars.next();
        }
    }
    result
}

pub fn format_edit(task: &Task) -> String {
    let mut text = String::new();
    // Keep whitespace intact while escaping tokens which the input parser owns.
    for token in task.title.split_inclusive(char::is_whitespace) {
        if token.starts_with('#') || token.trim_end() == "/p" || token.trim_end() == "/d" {
            text.push('\\');
        }
        text.push_str(&token.replace('\\', "\\\\"));
    }
    if let Some(date) = task.planned {
        text.push_str(&format!(" /p {date}"));
    }
    if let Some(date) = task.deadline {
        text.push_str(&format!(" /d {date}"));
    }
    for tag in &task.tags {
        text.push_str(&format!(" #{tag}"));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    fn date(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, day).unwrap()
    }

    #[test]
    fn les_metadonnees_sont_independantes() {
        // RG-F2-01/02 — Étant donné toutes les combinaisons de métadonnées.
        for flags in 0..8 {
            let mut raw = "Préparer  le\tsupport".to_owned();
            if flags & 1 != 0 {
                raw.push_str(" /p jeudi");
            }
            if flags & 2 != 0 {
                raw.push_str(" /d vendredi");
            }
            if flags & 4 != 0 {
                raw.push_str(" #mission");
            }
            // Quand la saisie est validée un jeudi.
            let input = parse_input(&raw, date(8)).unwrap();
            // Alors le titre reste intact et les métadonnées sont facultatives.
            assert_eq!(input.title, "Préparer  le\tsupport");
            assert_eq!(input.planned, (flags & 1 != 0).then_some(date(8)));
            assert_eq!(input.deadline, (flags & 2 != 0).then_some(date(9)));
            assert_eq!(input.tags.len(), usize::from(flags & 4 != 0));
        }
    }

    #[test]
    fn seules_les_commandes_explicites_sont_interpretees() {
        // RG-F2-02 — Étant donné des mots ressemblant à une date ou à une commande.
        let raw = "  Préparer vendredi /x demain dossier/p #fond suite  ";
        // Quand ils sont parsés.
        let input = parse_input(raw, date(9)).unwrap();
        // Alors seul le tag explicite est extrait, sans objet « fond » particulier.
        assert_eq!(input.title, "Préparer vendredi /x demain dossier/p suite");
        assert_eq!(input.tags, ["fond"]);
        assert_eq!(input.planned, None);
    }

    #[test]
    fn les_tokens_echappes_restent_litteraux() {
        // RG-F2-02 — Étant donné des commandes échappées.
        let input = parse_input(r"Relire \#mission \/p vendredi \/d demain", date(9)).unwrap();
        // Quand la saisie est validée, alors aucune métadonnée n'est créée.
        assert_eq!(input.title, "Relire #mission /p vendredi /d demain");
        assert!(input.tags.is_empty());
        assert_eq!(input.planned, None);
        assert_eq!(input.deadline, None);
    }

    #[test]
    fn demain_est_samedi_meme_si_le_prochain_jour_ouvre_est_lundi() {
        // RG-F2-03 — Étant donné un vendredi.
        let vendredi = date(9);
        // Quand vendredi et demain sont saisis.
        let input = parse_input(
            "Préparer le support /p vendredi /d demain #mission",
            vendredi,
        )
        .unwrap();
        // Alors vendredi est aujourd'hui et demain est samedi.
        assert_eq!(input.planned, Some(vendredi));
        assert_eq!(input.deadline, Some(date(10)));
        assert_eq!(input.title, "Préparer le support");
        assert_eq!(input.tags, ["mission"]);
    }

    #[test]
    fn les_dates_francaises_suivent_le_calendrier() {
        // RG-F2-03 — Étant donné chacun des sept jours de semaine.
        let names = [
            "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche",
        ];
        for current in 0..7 {
            let today = date(5 + current);
            // Quand chaque jour, aujourd'hui et demain sont demandés.
            for (target, name) in names.iter().enumerate() {
                let input = parse_input(&format!("Tâche /p {name}"), today).unwrap();
                // Alors la prochaine occurrence inclut aujourd'hui.
                assert_eq!(
                    input.planned,
                    Some(date(5 + current + (target as u32 + 7 - current) % 7))
                );
            }
            assert_eq!(
                parse_input("Tâche /p aujourd'hui", today).unwrap().planned,
                Some(today)
            );
            assert_eq!(
                parse_input("Tâche /p demain", today).unwrap().planned,
                Some(date(6 + current))
            );
        }
    }

    #[test]
    fn les_dates_calendaires_sont_validees() {
        // RG-F2-03/04 — Étant donné des dates calendaires possibles ou impossibles.
        for (raw, valid) in [
            ("2028-02-29", true),
            ("2026-02-29", false),
            ("2026-13-01", false),
            ("2026-2-01", false),
            ("09/10/2026", false),
        ] {
            // Quand une commande les utilise, alors seule une date stricte valide passe.
            assert_eq!(
                parse_input(&format!("Tâche /p {raw}"), date(9)).is_ok(),
                valid,
                "{raw}"
            );
        }
    }

    #[test]
    fn les_commandes_vides_repetees_ou_sans_titre_sont_refusees() {
        // RG-F2-04 — Étant donné des saisies ambiguës ou incomplètes.
        for raw in [
            "",
            " \t ",
            "Tâche /d",
            "Tâche /p bientôt",
            "Tâche /d /p lundi",
            "Tâche /p lundi /p mardi",
            "Tâche /d lundi /d mardi",
            "/p lundi #mission",
        ] {
            // Quand elles sont validées, alors une erreur compréhensible est produite.
            assert!(
                !parse_input(raw, date(9))
                    .unwrap_err()
                    .to_string()
                    .is_empty(),
                "{raw}"
            );
        }
    }

    #[test]
    fn les_tags_sont_valides_et_uniques_sans_casse() {
        // RG-F2-06 — Étant donné des tags Unicode et des graphies répétées.
        let input = parse_input(
            "Tâche #Mission #mission #Équipe-2 #équipe-2 #suivi_doc #42",
            date(9),
        )
        .unwrap();
        // Quand ils sont extraits, alors la première graphie est conservée une fois.
        assert_eq!(input.tags, ["Mission", "Équipe-2", "suivi_doc", "42"]);
        for raw in ["Tâche #", "Tâche #mission!", "Tâche #a/b"] {
            assert!(matches!(
                parse_input(raw, date(9)),
                Err(InputError::InvalidTag(_))
            ));
        }
    }

    #[test]
    fn une_edition_sans_changement_preserve_les_tokens_et_antislashs() {
        // RG-F2-02/05 — Étant donné des titres littéraux et des dates existantes.
        for title in [
            r"Relire #mission /p vendredi /d demain",
            r"Sauver \\serveur\dossier \#mission",
            "Titre  avec\tdes espaces",
            "#",
            r"\\#mission",
            "fin\\",
        ] {
            let task = Task {
                title: title.into(),
                planned: Some(date(9)),
                deadline: Some(date(12)),
                tags: vec!["Mission".into()],
                completed: false,
                completed_date: None,
                observed_completion: None,
            };
            // Quand l'édition est rouverte plusieurs jours après sa création.
            let formatted = format_edit(&task);
            let input = parse_input(&formatted, date(20)).unwrap();
            // Alors les valeurs absolues et le titre sont préservés exactement.
            assert_eq!(input.title, title, "{formatted}");
            assert_eq!(input.planned, task.planned);
            assert_eq!(input.deadline, task.deadline);
            assert_eq!(input.tags, task.tags);
        }
    }
}
