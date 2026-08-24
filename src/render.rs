use crate::store::{Event, Kind, Store};
use chrono::{Datelike, NaiveDate, Weekday};

const RESET: &str = "\x1b[0m";
const DIM: &str = "\x1b[2m";
const BOLD: &str = "\x1b[1m";
const REVERSE: &str = "\x1b[7m";
/// Weekends get their own color, distinct from every event kind.
const WEEKEND: &str = "\x1b[90m";

/// Visible width of one month block: 7 columns of 3 characters.
const BLOCK_WIDTH: usize = 21;

/// Width of the date column in listings: `2026-12-28 → 2026-12-30 (10d)`.
const DATES_WIDTH: usize = 30;

/// Renders one month as `BLOCK_WIDTH`-wide lines: title, weekday header, 6 week rows.
fn month_block(store: &Store, first: NaiveDate, today: NaiveDate, color: bool) -> Vec<String> {
    let title = first.format("%B %Y").to_string();
    let pad = BLOCK_WIDTH.saturating_sub(title.chars().count()) / 2;
    let mut lines = vec![
        format!("{:pad$}{}{}{}", "", paint(BOLD, color), title, reset(color)),
        format!("{}Mo Tu We Th Fr Sa Su{}", paint(DIM, color), reset(color)),
    ];

    let last = crate::cli::last_of_month(first);
    let mut day = first;
    for _ in 0..6 {
        let mut row = String::new();
        for column in 0..7 {
            let in_week = day <= last && weekday_index(day) == column;
            if in_week {
                row.push_str(&cell(store, day, today, color));
                day = day.succ_opt().unwrap_or(day);
            } else {
                row.push_str("   ");
            }
        }
        lines.push(row.trim_end().to_owned());
        if day > last {
            // Keep every block the same height so year view columns stay aligned.
            lines.resize(8, String::new());
            break;
        }
    }
    lines
}

/// Day number, 3 visible characters wide, colored by its kind.
fn cell(store: &Store, day: NaiveDate, today: NaiveDate, color: bool) -> String {
    let number = format!("{:>2} ", day.day());
    if !color {
        return number;
    }
    let mut style = String::new();
    if day == today {
        style.push_str(REVERSE);
        style.push_str(BOLD);
    }
    match store.day_kind(day) {
        Some(kind) => style.push_str(kind.color()),
        None if is_weekend(day) => style.push_str(WEEKEND),
        None => {}
    }
    if style.is_empty() {
        number
    } else {
        format!("{style}{:>2}{RESET} ", day.day())
    }
}

/// Every month touched by the range, three grids per row, plus the legend.
pub fn print_range(
    store: &Store,
    from: NaiveDate,
    to: NaiveDate,
    today: NaiveDate,
    color: bool,
    title: Option<&str>,
) {
    let blocks: Vec<Vec<String>> = months_between(from, to)
        .map(|first| month_block(store, first, today, color))
        .collect();

    if let Some(title) = title {
        let width = BLOCK_WIDTH * blocks.len().min(3) + 2 * blocks.len().min(3).saturating_sub(1);
        println!("{title:^width$}");
    }
    for row in blocks.chunks(3) {
        for line in 0..8 {
            let cells: Vec<String> = row
                .iter()
                .map(|block| pad_visible(block.get(line).map_or("", String::as_str)))
                .collect();
            println!("{}", cells.join("  ").trim_end());
        }
        if blocks.len() > 3 {
            println!();
        }
    }
    print_legend(store, from, to, color);
}

/// First day of every month between `from` and `to`, inclusive.
fn months_between(from: NaiveDate, to: NaiveDate) -> impl Iterator<Item = NaiveDate> {
    let last = crate::cli::first_of_month(to);
    std::iter::successors(Some(crate::cli::first_of_month(from)), |first| {
        first.checked_add_months(chrono::Months::new(1))
    })
    .take_while(move |first| *first <= last)
}

/// Pads a possibly colored line to `BLOCK_WIDTH` visible characters.
fn pad_visible(line: &str) -> String {
    let visible = visible_width(line);
    format!(
        "{line}{:width$}",
        "",
        width = BLOCK_WIDTH.saturating_sub(visible)
    )
}

fn visible_width(line: &str) -> usize {
    let mut width = 0;
    let mut chars = line.chars();
    while let Some(character) = chars.next() {
        if character == '\x1b' {
            for escaped in chars.by_ref() {
                if escaped == 'm' {
                    break;
                }
            }
        } else {
            width += 1;
        }
    }
    width
}

fn print_legend(store: &Store, from: NaiveDate, to: NaiveDate, color: bool) {
    let events = store.in_range(from, to);
    if events.is_empty() {
        println!("\nno events in range");
        return;
    }
    println!();
    for event in events {
        println!("  {}", event_line(event, color));
    }
    let workdays = days(from, to).filter(|day| store.is_working(*day)).count();
    let total = days(from, to).count();
    let events = store.in_range(from, to).len();
    println!(
        "\n{events} event{}, {workdays} working days, {} days off",
        if events == 1 { "" } else { "s" },
        total - workdays
    );
}

/// `uid  kind  dates  summary`, one event per line.
pub fn event_line(event: &Event, color: bool) -> String {
    let dates = if event.start == event.end {
        event.start.to_string()
    } else {
        format!("{} → {} ({}d)", event.start, event.end, event.len_days())
    };
    format!(
        "{}{:<8}{} {}{:<9}{} {dates:<DATES_WIDTH$}  {}",
        paint(DIM, color),
        &event.uid[..event.uid.len().min(8)],
        reset(color),
        paint(event.kind.color(), color),
        event.kind.to_string(),
        reset(color),
        event.summary
    )
}

pub fn print_list(events: &[&Event], color: bool) {
    if events.is_empty() {
        println!("no events in range");
        return;
    }
    println!(
        "{}{:<8} {:<9} {:<DATES_WIDTH$}  SUMMARY{}",
        paint(DIM, color),
        "UID",
        "KIND",
        "DATES",
        reset(color)
    );
    for event in events {
        println!("{}", event_line(event, color));
    }
}

pub fn is_weekend(day: NaiveDate) -> bool {
    matches!(day.weekday(), Weekday::Sat | Weekday::Sun)
}

/// Inclusive day iterator.
pub fn days(from: NaiveDate, to: NaiveDate) -> impl Iterator<Item = NaiveDate> {
    std::iter::successors(Some(from), NaiveDate::succ_opt).take_while(move |day| *day <= to)
}

fn weekday_index(day: NaiveDate) -> usize {
    day.weekday().num_days_from_monday() as usize
}

const fn paint(code: &str, color: bool) -> &str {
    if color { code } else { "" }
}

const fn reset(color: bool) -> &'static str {
    if color { RESET } else { "" }
}

/// Legend of what the colors mean.
pub fn print_key(color: bool) {
    let mut parts: Vec<String> = Kind::ALL
        .iter()
        .map(|kind| format!("{}{kind}{}", paint(kind.color(), color), reset(color)))
        .collect();
    parts.push(format!("{}weekend{}", paint(WEEKEND, color), reset(color)));
    println!(
        "{}key: {}{}",
        paint(DIM, color),
        parts.join(" "),
        reset(color)
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn month_block_is_aligned_and_complete() {
        let store = Store::load(PathBuf::from("/nonexistent/calmemo.ics")).unwrap();
        let first = NaiveDate::from_ymd_opt(2026, 8, 1).unwrap();
        let block = month_block(&store, first, first, false);
        assert_eq!(block.len(), 8, "title + header + 6 week rows");
        assert_eq!(block[1].trim_end(), "Mo Tu We Th Fr Sa Su");
        // 2026-08-01 is a Saturday: five leading blank columns.
        assert_eq!(block[2], "                1  2");
        let rendered = block.join(" ");
        for day in 1..=31 {
            assert!(rendered.contains(&format!("{day:>2}")), "day {day} missing");
        }
    }

    #[test]
    fn visible_width_ignores_ansi_codes() {
        assert_eq!(visible_width("\x1b[31m 1\x1b[0m "), 3);
    }
}
