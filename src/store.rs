use anyhow::{Context, Result, bail};
use chrono::{Datelike, NaiveDate, Weekday};
use clap::ValueEnum;
use icalendar::{Calendar, Component, DatePerhapsTime, Event as IcalEvent, EventLike};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// Category stored in the `CATEGORIES` property of every event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Kind {
    /// Public holiday: non-working even on a weekday.
    Holiday,
    /// Planned time off.
    Vacation,
    /// Sick leave.
    Sick,
    /// Any other non-working day, the default for new events.
    Dayoff,
    /// Forced working day, overrides weekends and days off.
    Workday,
    /// Plain note, does not affect working days.
    Note,
}

impl Kind {
    /// Kind used when an event does not say what type of day it is.
    pub const DEFAULT: Self = Self::Dayoff;

    pub const ALL: [Self; 6] = [
        Self::Holiday,
        Self::Vacation,
        Self::Sick,
        Self::Dayoff,
        Self::Workday,
        Self::Note,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Holiday => "holiday",
            Self::Vacation => "vacation",
            Self::Sick => "sick",
            Self::Dayoff => "dayoff",
            Self::Workday => "workday",
            Self::Note => "note",
        }
    }

    /// Does this kind make a day non-working?
    pub const fn is_off(self) -> bool {
        matches!(
            self,
            Self::Holiday | Self::Vacation | Self::Sick | Self::Dayoff
        )
    }

    /// ANSI color used when rendering days of this kind. Every kind, plus
    /// weekends, gets its own color.
    pub const fn color(self) -> &'static str {
        match self {
            Self::Holiday => "\x1b[31m",
            Self::Vacation => "\x1b[36m",
            Self::Sick => "\x1b[35m",
            Self::Dayoff => "\x1b[33m",
            Self::Workday => "\x1b[32m",
            Self::Note => "\x1b[34m",
        }
    }

    fn from_category(value: &str) -> Option<Self> {
        let value = value.trim().to_ascii_lowercase();
        match value.as_str() {
            "holiday" | "holidays" | "public holiday" | "public_holiday" => Some(Self::Holiday),
            "vacation" | "pto" | "leave" | "annual leave" => Some(Self::Vacation),
            "sick" | "sick leave" | "sickleave" | "sick_leave" => Some(Self::Sick),
            "dayoff" | "day off" | "day-off" | "day_off" | "off" => Some(Self::Dayoff),
            "workday" | "working day" => Some(Self::Workday),
            "note" => Some(Self::Note),
            _ => None,
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Kind {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Self::from_category(input)
            .ok_or_else(|| format!("unknown kind `{input}`, expected one of {}", Self::names()))
    }
}

impl Kind {
    fn names() -> String {
        Self::ALL.map(Self::as_str).join(", ")
    }
}

/// An all-day event spanning `start..=end`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub uid: String,
    pub summary: String,
    pub start: NaiveDate,
    pub end: NaiveDate,
    pub kind: Kind,
}

impl Event {
    pub fn new(summary: String, start: NaiveDate, end: NaiveDate, kind: Kind) -> Self {
        Self {
            uid: uuid::Uuid::new_v4().to_string(),
            summary,
            start,
            end,
            kind,
        }
    }

    /// Number of calendar days the event spans.
    pub fn len_days(&self) -> i64 {
        (self.end - self.start).num_days() + 1
    }

    fn to_ical(&self) -> IcalEvent {
        IcalEvent::new()
            .uid(&self.uid)
            .summary(&self.summary)
            .starts(self.start)
            // DTEND of an all-day event is exclusive.
            .ends(self.end.succ_opt().unwrap_or(self.end))
            .add_multi_property("CATEGORIES", self.kind.as_str())
            .add_property("TRANSP", "TRANSPARENT")
            .done()
    }

    fn from_ical(event: &IcalEvent, fallback: Kind) -> Option<Self> {
        let start = event.get_start()?.date_naive();
        let end = match event.get_end() {
            // Exclusive when the end is a bare DATE, inclusive otherwise.
            Some(DatePerhapsTime::Date(date)) => date.pred_opt().unwrap_or(date),
            Some(other) => other.date_naive(),
            None => start,
        };
        let kind = event
            .multi_properties()
            .get("CATEGORIES")
            .into_iter()
            .flatten()
            .flat_map(|property| property.value().split(','))
            .find_map(Kind::from_category)
            .unwrap_or(fallback);
        let end = end.max(start);
        let summary = event.get_summary().unwrap_or("(untitled)").to_owned();
        Some(Self {
            uid: event
                .get_uid()
                .map_or_else(|| format!("{start}-{end}-{summary}"), ToOwned::to_owned),
            summary,
            start,
            end,
            kind,
        })
    }
}

/// The on-disk calendar.
pub struct Store {
    path: PathBuf,
    pub events: Vec<Event>,
}

/// Name of the calendar used when `--name` is not given.
pub const DEFAULT_CALENDAR: &str = "default";

/// `$XDG_CONFIG_HOME/calmemo/<name>.ics`, falling back to `~/.config`.
pub fn path_for(name: &str) -> Result<PathBuf> {
    anyhow::ensure!(
        !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.'))
            && !name.starts_with('.'),
        "invalid calendar name `{name}`: use letters, digits, `-`, `_` or `.`"
    );
    let base = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => PathBuf::from(std::env::var_os("HOME").context("neither XDG_CONFIG_HOME nor HOME")?)
            .join(".config"),
    };
    Ok(base.join("calmemo").join(format!("{name}.ics")))
}

/// Names of the calendars that exist in the calmemo directory.
pub fn list_calendars() -> Result<Vec<String>> {
    let dir = path_for(DEFAULT_CALENDAR)?
        .parent()
        .expect("calendar path always has a parent")
        .to_path_buf();
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(error).with_context(|| format!("failed to read {}", dir.display()));
        }
    };
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "ics"))
        .filter_map(|path| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .map(ToOwned::to_owned)
        })
        .collect();
    names.sort();
    Ok(names)
}

impl Store {
    /// Loads the calendar, treating a missing file as empty.
    pub fn load(path: PathBuf) -> Result<Self> {
        let events = match fs::read_to_string(&path) {
            Ok(text) => parse_events(&text, Kind::DEFAULT)
                .with_context(|| format!("failed to parse {}", path.display()))?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(error) => {
                return Err(error).with_context(|| format!("failed to read {}", path.display()));
            }
        };
        Ok(Self { path, events })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Calendar name, i.e. the file stem of its path.
    pub fn name(&self) -> &str {
        self.path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or(DEFAULT_CALENDAR)
    }

    /// Writes the calendar through a temporary file and rename.
    pub fn save(&mut self) -> Result<()> {
        self.events
            .sort_by(|a, b| (a.start, a.end, &a.summary).cmp(&(b.start, b.end, &b.summary)));
        let mut calendar = Calendar::new();
        calendar.name(self.name());
        for event in &self.events {
            calendar.push(event.to_ical());
        }
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let temp = self.path.with_extension("ics.tmp");
        fs::write(&temp, calendar.to_string())
            .with_context(|| format!("failed to write {}", temp.display()))?;
        fs::rename(&temp, &self.path)
            .with_context(|| format!("failed to replace {}", self.path.display()))
    }

    /// Events overlapping the inclusive range, in chronological order.
    pub fn in_range(&self, from: NaiveDate, to: NaiveDate) -> Vec<&Event> {
        let mut found: Vec<&Event> = self
            .events
            .iter()
            .filter(|event| event.start <= to && from <= event.end)
            .collect();
        found.sort_by_key(|event| (event.start, event.end));
        found
    }

    pub fn on_day(&self, day: NaiveDate) -> Vec<&Event> {
        self.in_range(day, day)
    }

    /// The kind that decides the color/status of a day, if any event covers it.
    pub fn day_kind(&self, day: NaiveDate) -> Option<Kind> {
        // Workday wins over off-kinds, off-kinds win over notes.
        self.on_day(day)
            .iter()
            .map(|event| event.kind)
            .min_by_key(|kind| match kind {
                Kind::Workday => 0,
                Kind::Holiday => 1,
                Kind::Vacation => 2,
                Kind::Sick => 3,
                Kind::Dayoff => 4,
                Kind::Note => 5,
            })
    }

    /// A day is off on weekends and on holiday/vacation/sick events, unless a
    /// `workday` event forces it to be working.
    pub fn is_working(&self, day: NaiveDate) -> bool {
        let kinds = self.on_day(day);
        if kinds.iter().any(|event| event.kind == Kind::Workday) {
            return true;
        }
        if kinds.iter().any(|event| event.kind.is_off()) {
            return false;
        }
        !matches!(day.weekday(), Weekday::Sat | Weekday::Sun)
    }

    /// Why a day is off, for `offdays --reason`.
    pub fn off_reason(&self, day: NaiveDate) -> String {
        let events: Vec<String> = self
            .on_day(day)
            .iter()
            .filter(|event| event.kind.is_off())
            .map(|event| format!("{}: {}", event.kind, event.summary))
            .collect();
        if events.is_empty() {
            "weekend".to_owned()
        } else {
            events.join(", ")
        }
    }

    /// Finds one event by unique UID prefix.
    pub fn find_mut(&mut self, prefix: &str) -> Result<&mut Event> {
        let matches: Vec<usize> = self
            .events
            .iter()
            .enumerate()
            .filter(|(_, event)| event.uid.starts_with(prefix))
            .map(|(index, _)| index)
            .collect();
        match matches.as_slice() {
            [index] => Ok(&mut self.events[*index]),
            [] => bail!("no event with UID starting with `{prefix}`"),
            many => bail!("`{prefix}` matches {} events, be more specific", many.len()),
        }
    }

    /// Removes one event by unique UID prefix.
    pub fn remove(&mut self, prefix: &str) -> Result<Event> {
        let uid = self.find_mut(prefix)?.uid.clone();
        let index = self
            .events
            .iter()
            .position(|event| event.uid == uid)
            .expect("uid was just found");
        Ok(self.events.remove(index))
    }
}

/// Parses events out of an iCalendar document.
pub fn parse_events(text: &str, fallback: Kind) -> Result<Vec<Event>> {
    let calendar = Calendar::from_str(text).map_err(|error| anyhow::anyhow!(error))?;
    Ok(calendar
        .components
        .iter()
        .filter_map(|component| component.as_event())
        .filter_map(|event| Event::from_ical(event, fallback))
        .collect())
}

/// Parses `start[;end];[kind];summary` bulk input lines.
pub fn parse_bulk(text: &str, fallback: Kind) -> Result<Vec<Event>> {
    let mut events = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split(';').map(str::trim).collect();
        let parse = |field: &str| {
            NaiveDate::parse_from_str(field, "%Y-%m-%d")
                .with_context(|| format!("line {}: invalid date `{field}`", number + 1))
        };
        let (start, end, kind, summary) = match fields.as_slice() {
            [start, end, kind, summary @ ..] if fields.len() > 3 => (
                parse(start)?,
                if end.is_empty() {
                    parse(start)?
                } else {
                    parse(end)?
                },
                *kind,
                summary.join(";"),
            ),
            [start, kind, summary @ ..] => (parse(start)?, parse(start)?, *kind, summary.join(";")),
            _ => bail!("line {}: expected `start;end;kind;summary`", number + 1),
        };
        let kind = if kind.is_empty() {
            fallback
        } else {
            kind.parse()
                .map_err(|error| {
                    let hint = if fields.len() == 3 {
                        "; three-field lines are `start;kind;summary`; use `start;end;kind;summary` for date ranges"
                    } else {
                        ""
                    };
                    anyhow::anyhow!("line {}: {error}{hint}", number + 1)
                })?
        };
        if end < start {
            bail!("line {}: end {end} is before start {start}", number + 1);
        }
        let summary = if summary.trim().is_empty() {
            kind.as_str().to_owned()
        } else {
            summary.trim().to_owned()
        };
        events.push(Event::new(summary, start, end, kind));
    }
    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(text: &str) -> NaiveDate {
        text.parse().unwrap()
    }

    fn store(events: Vec<Event>) -> Store {
        Store {
            path: PathBuf::from("/dev/null"),
            events,
        }
    }

    #[test]
    fn weekends_and_events_decide_working_days() {
        let store = store(vec![
            // Friday holiday, Saturday forced workday.
            Event::new(
                "Xmas".into(),
                date("2026-12-25"),
                date("2026-12-25"),
                Kind::Holiday,
            ),
            Event::new(
                "catch up".into(),
                date("2026-12-26"),
                date("2026-12-26"),
                Kind::Workday,
            ),
            Event::new(
                "ski".into(),
                date("2026-12-28"),
                date("2026-12-30"),
                Kind::Vacation,
            ),
            Event::new(
                "dentist".into(),
                date("2026-12-31"),
                date("2026-12-31"),
                Kind::Note,
            ),
            Event::new(
                "moving".into(),
                date("2026-12-24"),
                date("2026-12-24"),
                Kind::DEFAULT,
            ),
        ]);
        assert_eq!(Kind::DEFAULT, Kind::Dayoff, "unspecified kind is a day off");
        assert!(!store.is_working(date("2026-12-24")), "generic day off");
        assert!(!store.is_working(date("2026-12-25")), "holiday");
        assert!(
            store.is_working(date("2026-12-26")),
            "forced workday on Saturday"
        );
        assert!(!store.is_working(date("2026-12-27")), "plain Sunday");
        assert!(!store.is_working(date("2026-12-28")), "vacation");
        assert!(
            store.is_working(date("2026-12-31")),
            "notes do not take a day off"
        );
        assert_eq!(store.off_reason(date("2026-12-27")), "weekend");
        assert_eq!(store.off_reason(date("2026-12-25")), "holiday: Xmas");
    }

    #[test]
    fn ical_roundtrip_keeps_kind_and_inclusive_end() {
        let original = Event::new(
            "ski".into(),
            date("2026-12-28"),
            date("2026-12-30"),
            Kind::Vacation,
        );
        let mut calendar = Calendar::new();
        calendar.push(original.to_ical());
        let parsed = parse_events(&calendar.to_string(), Kind::Note).unwrap();
        assert_eq!(parsed, vec![original]);
    }

    #[test]
    fn missing_ical_uid_is_stable() {
        let text = concat!(
            "BEGIN:VCALENDAR\r\nVERSION:2.0\r\n",
            "BEGIN:VEVENT\r\n",
            "DTSTART;VALUE=DATE:20260201\r\n",
            "DTEND;VALUE=DATE:20260206\r\n",
            "SUMMARY:Ski\r\n",
            "END:VEVENT\r\nEND:VCALENDAR\r\n",
        );
        let first = parse_events(text, Kind::Vacation).unwrap();
        let second = parse_events(text, Kind::Vacation).unwrap();
        assert_eq!(first, second);
        assert_eq!(first[0].uid, "2026-02-01-2026-02-05-Ski");
    }

    #[test]
    fn bulk_range_without_kind_explains_format() {
        let error = parse_bulk("2026-02-01;2026-02-05;Ski\n", Kind::Vacation)
            .unwrap_err()
            .to_string();
        assert!(error.contains("three-field lines are `start;kind;summary`"));
    }

    #[test]
    fn bulk_lines_accept_three_and_four_fields() {
        let events = parse_bulk(
            "# comment\n2026-01-01;;holiday;New Year\n2026-02-01;2026-02-05;vacation;Ski\n2026-03-01;sick;Flu\n",
            Kind::Holiday,
        )
        .unwrap();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].len_days(), 1);
        assert_eq!(events[1].len_days(), 5);
        assert_eq!(events[2].kind, Kind::Sick);
    }
}
