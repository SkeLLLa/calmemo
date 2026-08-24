use crate::store::Kind;
use anyhow::{Context, Result, bail};
use chrono::{Datelike, Local, NaiveDate};
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "calmemo",
    version,
    about = "Local calendar for holidays, vacations and working-day reports"
)]
pub struct Cli {
    /// Calendar to use, stored as `$XDG_CONFIG_HOME/calmemo/<NAME>.ics`
    #[arg(short = 'n', long, global = true, value_name = "NAME")]
    pub name: Option<String>,

    /// Calendar file to use, overrides `--name`
    #[arg(short = 'c', long = "calendar", global = true, value_name = "FILE")]
    pub calendar: Option<PathBuf>,

    /// Never colorize output
    #[arg(long, global = true)]
    pub no_color: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Render the calendar in the terminal (default command)
    #[command(visible_alias = "cal")]
    Show {
        #[command(flatten)]
        range: RangeArgs,
    },
    /// List events as a table
    #[command(visible_alias = "ls")]
    List {
        #[command(flatten)]
        range: RangeArgs,
        /// Only events of this kind (repeatable)
        #[arg(short, long = "kind")]
        kinds: Vec<Kind>,
    },
    /// Add a single event
    Add {
        /// First day of the event
        #[arg(short, long, value_name = "YYYY-MM-DD")]
        start: NaiveDate,
        /// Last day of the event, inclusive (default: same as start)
        #[arg(short, long, value_name = "YYYY-MM-DD")]
        end: Option<NaiveDate>,
        /// Type of day, e.g. which kind of non-working day this is
        #[arg(short, long, default_value = "dayoff")]
        kind: Kind,
        /// Event title
        #[arg(required = true, num_args = 1..)]
        summary: Vec<String>,
    },
    /// Add many events at once from `start[;end];kind;summary` lines
    Bulk {
        /// Input file, or `-` for stdin
        #[arg(default_value = "-")]
        file: PathBuf,
        /// Kind used for lines that omit it
        #[arg(short, long, default_value = "dayoff")]
        kind: Kind,
    },
    /// Merge events from another iCalendar file
    Import {
        file: PathBuf,
        /// Kind assigned to events without a known category
        #[arg(short, long, default_value = "dayoff")]
        kind: Kind,
    },
    /// Change an existing event, addressed by UID prefix
    Edit {
        uid: String,
        #[arg(short, long, value_name = "YYYY-MM-DD")]
        start: Option<NaiveDate>,
        #[arg(short, long, value_name = "YYYY-MM-DD")]
        end: Option<NaiveDate>,
        #[arg(short, long)]
        kind: Option<Kind>,
        #[arg(long, num_args = 1..)]
        summary: Option<Vec<String>>,
    },
    /// Delete events, addressed by UID prefix
    #[command(visible_alias = "rm")]
    Remove {
        #[arg(required = true)]
        uids: Vec<String>,
    },
    /// Print working days, one ISO date per line
    Workdays {
        #[command(flatten)]
        range: RangeArgs,
    },
    /// Print non-working days (weekends, holidays, vacations, sick leaves)
    #[command(visible_alias = "holidays")]
    Offdays {
        #[command(flatten)]
        range: RangeArgs,
        /// Append `<tab>reason` to every line
        #[arg(long)]
        reason: bool,
    },
    /// Print the path of the calendar file
    Path,
    /// List the calendars stored in the calmemo directory
    Calendars,
}

/// Time span selector shared by most commands.
#[derive(Debug, Default, Args)]
pub struct RangeArgs {
    /// Whole month, e.g. 2026-08
    #[arg(short, long, value_name = "YYYY-MM", conflicts_with_all = ["year", "from"])]
    pub month: Option<String>,
    /// Whole year, e.g. 2026
    #[arg(short, long, conflicts_with = "from")]
    pub year: Option<i32>,
    /// Range start, defaults to the first day of `--to`'s month
    #[arg(short, long, value_name = "YYYY-MM-DD")]
    pub from: Option<NaiveDate>,
    /// Range end, defaults to the last day of `--from`'s month
    #[arg(short, long, value_name = "YYYY-MM-DD")]
    pub to: Option<NaiveDate>,
}

impl RangeArgs {
    /// Resolves the selector into an inclusive date range. Without any
    /// selector this is the whole current month, past days included.
    pub fn resolve(&self) -> Result<(NaiveDate, NaiveDate)> {
        if let Some(month) = &self.month {
            let first = parse_month(month)?;
            return Ok((first, last_of_month(first)));
        }
        if let Some(year) = self.year {
            return Ok((ymd(year, 1, 1)?, ymd(year, 12, 31)?));
        }
        let (from, to) = match (self.from, self.to) {
            // A single open end covers the rest, or the beginning, of its month.
            (Some(from), None) => (from, last_of_month(from)),
            (None, Some(to)) => (first_of_month(to), to),
            (Some(from), Some(to)) => (from, to),
            (None, None) => {
                let first = first_of_month(Local::now().date_naive());
                (first, last_of_month(first))
            }
        };
        if to < from {
            bail!("--to {to} is before --from {from}");
        }
        Ok((from, to))
    }

    /// The year to render when a whole year was requested.
    pub const fn whole_year(&self) -> Option<i32> {
        self.year
    }
}

fn parse_month(input: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(&format!("{input}-01"), "%Y-%m-%d")
        .with_context(|| format!("invalid month `{input}`, expected YYYY-MM"))
}

fn ymd(year: i32, month: u32, day: u32) -> Result<NaiveDate> {
    NaiveDate::from_ymd_opt(year, month, day)
        .with_context(|| format!("invalid date {year}-{month:02}-{day:02}"))
}

/// First day of the month that `date` belongs to.
pub fn first_of_month(date: NaiveDate) -> NaiveDate {
    date.with_day(1).unwrap_or(date)
}

/// Last day of the month that `date` belongs to.
pub fn last_of_month(date: NaiveDate) -> NaiveDate {
    let (year, month) = if date.month() == 12 {
        (date.year() + 1, 1)
    } else {
        (date.year(), date.month() + 1)
    };
    NaiveDate::from_ymd_opt(year, month, 1)
        .and_then(|first| first.pred_opt())
        .unwrap_or(date)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn range(
        month: Option<&str>,
        year: Option<i32>,
        from: Option<&str>,
        to: Option<&str>,
    ) -> RangeArgs {
        RangeArgs {
            month: month.map(ToOwned::to_owned),
            year,
            from: from.map(|date| date.parse().unwrap()),
            to: to.map(|date| date.parse().unwrap()),
        }
    }

    fn date(text: &str) -> NaiveDate {
        text.parse().unwrap()
    }

    #[test]
    fn default_range_is_the_whole_current_month() {
        let today = Local::now().date_naive();
        let (from, to) = range(None, None, None, None).resolve().unwrap();
        assert_eq!(from, first_of_month(today));
        assert_eq!(
            to,
            last_of_month(today),
            "the whole month, not just up to today"
        );
    }

    #[test]
    fn selectors_and_open_ends() {
        assert_eq!(
            range(Some("2026-02"), None, None, None).resolve().unwrap(),
            (date("2026-02-01"), date("2026-02-28"))
        );
        assert_eq!(
            range(None, Some(2026), None, None).resolve().unwrap(),
            (date("2026-01-01"), date("2026-12-31"))
        );
        assert_eq!(
            range(None, None, Some("2026-08-20"), None)
                .resolve()
                .unwrap(),
            (date("2026-08-20"), date("2026-08-31")),
            "--from alone runs to the end of its month"
        );
        assert_eq!(
            range(None, None, None, Some("2026-08-20"))
                .resolve()
                .unwrap(),
            (date("2026-08-01"), date("2026-08-20")),
            "--to alone starts at the beginning of its month"
        );
        assert!(
            range(None, None, Some("2026-08-20"), Some("2026-08-01"))
                .resolve()
                .is_err(),
            "reversed range is rejected"
        );
        assert!(range(Some("2026-13"), None, None, None).resolve().is_err());
    }
}
