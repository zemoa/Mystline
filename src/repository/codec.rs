use crate::domain::{Task, same_tag, valid_tag};
use chrono::NaiveDate;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CodecError {
    #[error("ligne {line} invalide : {reason}")]
    Invalid { line: usize, reason: String },
}

pub fn parse(text: &str, today: NaiveDate) -> Result<Vec<Task>, CodecError> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, line)| parse_line(line, index + 1, today))
        .collect()
}

fn parse_line(line: &str, number: usize, today: NaiveDate) -> Result<Task, CodecError> {
    let fail = |reason: &str| CodecError::Invalid {
        line: number,
        reason: reason.into(),
    };
    let (completed, body) = if let Some(body) = line.strip_prefix("- [ ] ") {
        (false, body)
    } else if let Some(body) = line.strip_prefix("- [x] ") {
        (true, body)
    } else {
        return Err(fail("case Markdown attendue"));
    };
    let mut title = String::new();
    let mut tags = Vec::new();
    let (mut planned, mut deadline, mut completed_date) = (None, None, None);
    // Older files treat backslashes literally. Only lines written with this
    // marker use escapes in their title; an application write migrates them.
    let escaped_title = body
        .split_whitespace()
        .any(|token| token == "@title-escaped(1)");
    let mut seen_marker = false;
    let mut previous_end = 0;
    let mut previous_was_title = false;
    let mut offset = 0;
    while offset < body.len() {
        let rest = &body[offset..];
        let skipped = rest.len() - rest.trim_start_matches(char::is_whitespace).len();
        offset += skipped;
        if offset == body.len() {
            break;
        }
        let start = offset;
        offset = body[start..]
            .find(char::is_whitespace)
            .map_or(body.len(), |length| start + length);
        let token = &body[start..offset];
        if token == "@title-escaped(1)" {
            if seen_marker {
                return Err(fail("métadonnée dupliquée"));
            }
            seen_marker = true;
            previous_was_title = false;
        } else if token.starts_with('\\') {
            if !title.is_empty() {
                title.push_str(if previous_was_title {
                    &body[previous_end..start]
                } else {
                    " "
                });
            }
            title.push_str(&decode_title_token(token, escaped_title));
            previous_was_title = true;
        } else if let Some(tag) = token.strip_prefix('#') {
            if !valid_tag(tag) {
                return Err(fail("tag invalide"));
            }
            if !tags.iter().any(|old: &String| same_tag(old, tag)) {
                tags.push(tag.to_owned());
            }
            previous_was_title = false;
        } else if token.starts_with('@') {
            let (slot, raw) = if let Some(raw) = token
                .strip_prefix("@planned(")
                .and_then(|s| s.strip_suffix(')'))
            {
                (&mut planned, raw)
            } else if let Some(raw) = token
                .strip_prefix("@deadline(")
                .and_then(|s| s.strip_suffix(')'))
            {
                (&mut deadline, raw)
            } else if let Some(raw) = token
                .strip_prefix("@completed(")
                .and_then(|s| s.strip_suffix(')'))
            {
                (&mut completed_date, raw)
            } else {
                return Err(fail("métadonnée inconnue"));
            };
            if slot.is_some() {
                return Err(fail("métadonnée dupliquée"));
            }
            if raw.len() != 10 {
                return Err(fail("date invalide"));
            }
            *slot = Some(
                NaiveDate::parse_from_str(raw, "%Y-%m-%d").map_err(|_| fail("date invalide"))?,
            );
            previous_was_title = false;
        } else {
            if !title.is_empty() {
                title.push_str(if previous_was_title {
                    &body[previous_end..start]
                } else {
                    " "
                });
            }
            title.push_str(&decode_title_token(token, escaped_title));
            previous_was_title = true;
        }
        previous_end = offset;
    }
    if title.is_empty() || (!completed && completed_date.is_some()) {
        return Err(fail("titre manquant ou date de fin sans complétion"));
    }
    Ok(Task {
        title,
        planned,
        deadline,
        tags,
        completed,
        completed_date,
        observed_completion: if completed && completed_date.is_none() {
            Some(today)
        } else {
            None
        },
    })
}

fn decode_title_token(token: &str, escaped_title: bool) -> String {
    if !escaped_title {
        return token.to_owned();
    }
    let token = token
        .strip_prefix("\\#")
        .map(|s| format!("#{s}"))
        .or_else(|| token.strip_prefix("\\@").map(|s| format!("@{s}")))
        .unwrap_or_else(|| token.to_owned());
    token.replace("\\\\", "\\")
}

pub fn serialize(tasks: &[Task]) -> String {
    let mut output = String::new();
    for task in tasks {
        output.push_str(if task.completed { "- [x] " } else { "- [ ] " });
        let mut at_token_start = true;
        let mut escaped_title = false;
        for character in task.title.chars() {
            if character.is_whitespace() {
                output.push(character);
                at_token_start = true;
            } else {
                if character == '\\' || (at_token_start && (character == '#' || character == '@')) {
                    output.push('\\');
                    escaped_title = true;
                }
                output.push(character);
                at_token_start = false;
            }
        }
        for tag in &task.tags {
            output.push_str(" #");
            output.push_str(tag);
        }
        if escaped_title {
            output.push_str(" @title-escaped(1)");
        }
        for (key, date) in [
            ("planned", task.planned),
            ("deadline", task.deadline),
            (
                "completed",
                if task.completed {
                    task.effective_completion_date()
                } else {
                    None
                },
            ),
        ] {
            if let Some(date) = date {
                output.push_str(&format!(" @{key}({date})"));
            }
        }
        output.push('\n');
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn round_trip_and_observed_completion() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 4).unwrap();
        let tasks = parse(
            "- [ ] Tester #mission @planned(2026-10-05)\n- [x] Terminé\n",
            today,
        )
        .unwrap();
        assert_eq!(tasks[1].effective_completion_date(), Some(today));
        assert_eq!(
            serialize(&tasks),
            "- [ ] Tester #mission @planned(2026-10-05)\n- [x] Terminé @completed(2026-10-04)\n"
        );
    }
    #[test]
    fn invalid_file_is_rejected_whole() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 4).unwrap();
        assert!(
            parse(
                "- [ ] Correct\n- [ ] Incorrect @deadline(2026-02-30)",
                today
            )
            .is_err()
        );
    }

    #[test]
    fn literal_reserved_tokens_and_backslashes_round_trip_without_losing_real_tags() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 4).unwrap();
        let mut task = parse("- [ ] Titre #mission @planned(2026-10-05)\n", today)
            .unwrap()
            .remove(0);
        task.title = r"Voir #architecture @deadline(2026-10-06) \#mission \@planned(2026-10-07) chemin\fichier".into();

        let markdown = serialize(&[task.clone()]);
        assert!(markdown.contains(r"\#architecture"));
        assert!(markdown.contains(r"\@deadline(2026-10-06)"));
        assert_eq!(parse(&markdown, today).unwrap(), vec![task]);
    }

    #[test]
    fn escaped_reserved_tokens_are_title_but_unescaped_metadata_is_still_metadata() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 4).unwrap();
        let tasks = parse(
            "- [ ] Bonjour  \\#architecture \\@deadline(2026-10-06) \\\\#littéral #mission @planned(2026-10-05) @title-escaped(1)\n",
            today,
        )
        .unwrap();
        assert_eq!(
            tasks[0].title,
            "Bonjour  #architecture @deadline(2026-10-06) \\#littéral"
        );
        assert_eq!(tasks[0].tags, ["mission"]);
        assert_eq!(tasks[0].planned, NaiveDate::from_ymd_opt(2026, 10, 5));
    }

    #[test]
    fn legacy_file_preserves_backslashes_without_title_escape_marker() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 4).unwrap();
        let legacy = "- [ ] Sauver \\\\serveur\\dossier et \\#mission #travail\n";
        let tasks = parse(legacy, today).unwrap();
        assert_eq!(tasks[0].title, "Sauver \\\\serveur\\dossier et \\#mission");
        assert_eq!(tasks[0].tags, ["travail"]);
        assert_eq!(parse(&serialize(&tasks), today).unwrap(), tasks);
    }
}
