use chrono::NaiveDate;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub title: String,
    pub planned: Option<NaiveDate>,
    pub deadline: Option<NaiveDate>,
    pub tags: Vec<String>,
    pub completed: bool,
    pub completed_date: Option<NaiveDate>,
    /// Date of observation, never serialized until an application-initiated commit.
    pub observed_completion: Option<NaiveDate>,
}

impl Task {
    pub fn effective_completion_date(&self) -> Option<NaiveDate> {
        self.completed_date.or(self.observed_completion)
    }
}
