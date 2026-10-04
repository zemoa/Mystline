use crate::domain::Task;
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
    let mut title = Vec::new();
    let mut tags = Vec::new();
    let (mut planned, mut deadline, mut completed_date) = (None, None, None);
    for token in body.split_whitespace() {
        if let Some(tag) = token.strip_prefix('#') {
            if tag.is_empty()
                || !tag
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
            {
                return Err(fail("tag invalide"));
            }
            tags.push(tag.to_owned());
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
        } else {
            title.push(token);
        }
    }
    if title.is_empty() || (!completed && completed_date.is_some()) {
        return Err(fail("titre manquant ou date de fin sans complétion"));
    }
    Ok(Task {
        title: title.join(" "),
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

pub fn serialize(tasks: &[Task]) -> String {
    let mut output = String::new();
    for task in tasks {
        output.push_str(if task.completed { "- [x] " } else { "- [ ] " });
        output.push_str(&task.title);
        for tag in &task.tags {
            output.push_str(" #");
            output.push_str(tag);
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
}
