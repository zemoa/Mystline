use crate::domain::{Task, same_tag, valid_tag};
use chrono::NaiveDate;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CodecError {
    #[error("ligne {line} invalide : {reason}")]
    Invalid { line: usize, reason: String },
}

pub fn parse(text: &str, today: NaiveDate) -> Result<Vec<Task>, CodecError> {
    let tasks: Vec<Task> = text
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, line)| parse_line(line, index + 1, today))
        .collect::<Result<_, _>>()?;
    let mut ids = std::collections::HashSet::new();
    let mut active = std::collections::HashSet::new();
    for (index, task) in tasks.iter().enumerate() {
        if let Some(id) = task.occurrence
            && (!ids.insert(id) || (!task.completed && !active.insert(id.series)))
        {
            return Err(CodecError::Invalid {
                line: index + 1,
                reason: "occurrence dupliquée ou plusieurs occurrences à faire dans une série"
                    .into(),
            });
        }
    }
    Ok(tasks)
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
    let (mut recurrence, mut occurrence) = (None, None);
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
        } else if let Some(raw) = token
            .strip_prefix("@repeat(")
            .and_then(|s| s.strip_suffix(')'))
        {
            if recurrence.is_some() {
                return Err(fail("répétition dupliquée"));
            }
            let parts = raw.split(';').collect::<Vec<_>>();
            if parts.len() != 4 {
                return Err(fail("répétition invalide"));
            }
            recurrence = Some(crate::recurrence::Recurrence {
                rule: crate::recurrence::RepeatRule::parse(parts[0])
                    .ok_or_else(|| fail("fréquence invalide"))?,
                reference: parts[1]
                    .parse()
                    .map_err(|_| fail("date théorique invalide"))?,
                anchor: parts[2].parse().map_err(|_| fail("ancrage invalide"))?,
                deadline_offset: if parts[3] == "none" {
                    None
                } else {
                    Some(
                        parts[3]
                            .parse()
                            .map_err(|_| fail("écart de deadline invalide"))?,
                    )
                },
            });
            previous_was_title = false;
        } else if let Some(raw) = token
            .strip_prefix("@occurrence(")
            .and_then(|s| s.strip_suffix(')'))
        {
            if occurrence.is_some() {
                return Err(fail("identifiant dupliqué"));
            }
            let (series, generation) = raw
                .split_once(':')
                .ok_or_else(|| fail("identifiant invalide"))?;
            occurrence = Some(crate::recurrence::OccurrenceId {
                series: series.parse().map_err(|_| fail("série invalide"))?,
                generation: generation
                    .parse()
                    .map_err(|_| fail("génération invalide"))?,
            });
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
    if let Some(r) = &recurrence
        && (occurrence.is_none()
            || planned.is_none()
            || !r.rule.compatible(r.reference)
            || !r.rule.compatible(r.anchor)
            || matches!(r.rule, crate::recurrence::RepeatRule::EveryDays(n) if (r.reference - r.anchor).num_days().rem_euclid(i64::from(n)) != 0)
            || r.deadline_offset.is_some_and(|n| {
                chrono::Duration::try_days(n)
                    .and_then(|d| r.reference.checked_add_signed(d))
                    .is_none()
            }))
    {
        return Err(fail("calendrier récurrent incohérent"));
    }
    Ok(Task {
        recurrence,
        occurrence,
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
        if let Some(id) = task.occurrence {
            output.push_str(&format!(" @occurrence({}:{})", id.series, id.generation));
        }
        if let Some(r) = &task.recurrence {
            output.push_str(&format!(
                " @repeat({};{};{};{})",
                r.rule,
                r.reference,
                r.anchor,
                r.deadline_offset
                    .map_or_else(|| "none".into(), |n| n.to_string())
            ));
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
