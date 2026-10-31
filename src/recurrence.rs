//! Calendriers locaux sans heure ; aucune génération au simple passage du temps.
use chrono::{Datelike, Days, NaiveDate};
use std::fmt;

pub const WEEKDAYS: [&str; 7] = [
    "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche",
];

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RepeatRule {
    Daily,
    Weekly(Vec<u32>),
    EveryDays(u32),
    Monthly(u32),
}

impl RepeatRule {
    pub fn parse(raw: &str) -> Option<Self> {
        let raw = raw.to_lowercase();
        if raw == "jour" {
            return Some(Self::Daily);
        }
        if let Some(day) = raw.strip_prefix("mois:") {
            let day = day.parse().ok()?;
            return (1..=31).contains(&day).then_some(Self::Monthly(day));
        }
        if let Some(days) = raw.strip_suffix('j') {
            let days = days.parse().ok()?;
            return (days > 0).then_some(Self::EveryDays(days));
        }
        let mut days = raw
            .split(',')
            .map(|name| {
                WEEKDAYS
                    .iter()
                    .position(|day| *day == name)
                    .map(|day| day as u32)
            })
            .collect::<Option<Vec<_>>>()?;
        days.sort_unstable();
        days.dedup();
        (!days.is_empty()).then_some(Self::Weekly(days))
    }

    pub fn valid(&self) -> bool {
        match self {
            Self::Daily => true,
            Self::EveryDays(n) => *n > 0,
            Self::Monthly(day) => (1..=31).contains(day),
            Self::Weekly(days) => !days.is_empty() && days.iter().all(|day| *day < 7),
        }
    }

    pub fn compatible(&self, date: NaiveDate) -> bool {
        match self {
            Self::Weekly(days) => days.contains(&date.weekday().num_days_from_monday()),
            Self::Monthly(day) => monthly_date(date.year(), date.month(), *day) == Some(date),
            _ => self.valid(),
        }
    }

    pub fn first(&self, today: NaiveDate) -> Option<NaiveDate> {
        self.on_or_after(today, today)
    }

    pub fn after(&self, day: NaiveDate, anchor: NaiveDate) -> Option<NaiveDate> {
        self.on_or_after(day.succ_opt()?, anchor)
    }

    fn on_or_after(&self, day: NaiveDate, anchor: NaiveDate) -> Option<NaiveDate> {
        if !self.valid() {
            return None;
        }
        match self {
            Self::Daily => Some(day),
            Self::EveryDays(n) => {
                let distance = (day - anchor).num_days();
                let remainder = distance.rem_euclid(i64::from(*n));
                let offset = (i64::from(*n) - remainder) % i64::from(*n);
                day.checked_add_days(Days::new(offset as u64))
            }
            Self::Weekly(_) => (0..7)
                .filter_map(|n| day.checked_add_days(Days::new(n)))
                .find(|date| self.compatible(*date)),
            Self::Monthly(wanted) => {
                let candidate = monthly_date(day.year(), day.month(), *wanted)?;
                if candidate >= day {
                    Some(candidate)
                } else {
                    let (year, month) = if day.month() == 12 {
                        (day.year().checked_add(1)?, 1)
                    } else {
                        (day.year(), day.month() + 1)
                    };
                    monthly_date(year, month, *wanted)
                }
            }
        }
    }
}

fn monthly_date(year: i32, month: u32, wanted: u32) -> Option<NaiveDate> {
    (1..=wanted.min(31))
        .rev()
        .find_map(|day| NaiveDate::from_ymd_opt(year, month, day))
}

impl fmt::Display for RepeatRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Daily => f.write_str("jour"),
            Self::EveryDays(n) => write!(f, "{n}j"),
            Self::Monthly(day) => write!(f, "mois:{day}"),
            Self::Weekly(days) => f.write_str(
                &days
                    .iter()
                    .map(|day| WEEKDAYS[*day as usize])
                    .collect::<Vec<_>>()
                    .join(","),
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Recurrence {
    pub rule: RepeatRule,
    pub reference: NaiveDate,
    pub anchor: NaiveDate,
    pub deadline_offset: Option<i64>,
}

/// Le lien survit à « Ne plus répéter » pour permettre l'annulation du cochage précédent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OccurrenceId {
    pub series: u64,
    pub generation: u64,
}

pub fn next_occurrence(task: &crate::domain::Task) -> Option<crate::domain::Task> {
    let recurrence = task.recurrence.as_ref()?;
    let id = task.occurrence?;
    let reference = recurrence.rule.after(
        task.effective_completion_date()?.max(recurrence.reference),
        recurrence.anchor,
    )?;
    let mut next = task.clone();
    next.completed = false;
    next.completed_date = None;
    next.observed_completion = None;
    next.planned = Some(reference);
    next.deadline = match recurrence.deadline_offset {
        Some(offset) => Some(reference.checked_add_signed(chrono::Duration::try_days(offset)?)?),
        None => None,
    };
    next.recurrence.as_mut()?.reference = reference;
    next.occurrence = Some(OccurrenceId {
        series: id.series,
        generation: id.generation.checked_add(1)?,
    });
    Some(next)
}

/// Appelé seulement par une écriture applicative, jamais par le watcher.
pub fn generate_missing(tasks: &mut Vec<crate::domain::Task>) -> Result<(), &'static str> {
    let mut latest = std::collections::HashMap::new();
    for task in tasks.iter() {
        if let Some(id) = task.occurrence {
            latest
                .entry(id.series)
                .and_modify(|n: &mut u64| *n = (*n).max(id.generation))
                .or_insert(id.generation);
        }
    }
    let mut next = Vec::new();
    for task in tasks.iter() {
        if task.completed
            && task.recurrence.is_some()
            && task
                .occurrence
                .is_some_and(|id| latest.get(&id.series) == Some(&id.generation))
        {
            next.push(
                next_occurrence(task)
                    .ok_or("La prochaine date dépasse le calendrier pris en charge.")?,
            );
        }
    }
    tasks.extend(next);
    Ok(())
}
