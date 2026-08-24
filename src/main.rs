#![doc = include_str!("../docs/index.md")]

#[doc = include_str!("../docs/getting-started.md")]
pub mod getting_started {}

#[doc = include_str!("../docs/development.md")]
pub mod development {}

mod cli;
mod render;
mod store;

use anyhow::{Context, Result};
use chrono::Local;
use clap::Parser;
use cli::{Cli, Command};
use std::io::{IsTerminal, Read};
use std::path::Path;
use store::{Event, Store};

fn main() -> Result<()> {
    let cli = Cli::parse();
    let color = !cli.no_color
        && std::env::var_os("NO_COLOR").is_none()
        && (std::io::stdout().is_terminal() || std::env::var_os("FORCE_COLOR").is_some());
    let path = match cli.calendar {
        Some(path) => path,
        None => store::path_for(cli.name.as_deref().unwrap_or(store::DEFAULT_CALENDAR))?,
    };
    let mut store = Store::load(path)?;
    let today = Local::now().date_naive();

    match cli.command.unwrap_or_else(|| Command::Show {
        range: cli::RangeArgs::default(),
    }) {
        Command::Show { range } => {
            let title = range.whole_year().map(|year| year.to_string());
            let (from, to) = range.resolve()?;
            render::print_range(&store, from, to, today, color, title.as_deref());
            render::print_key(color);
        }
        Command::List { range, kinds } => {
            let (from, to) = range.resolve()?;
            let events: Vec<&Event> = store
                .in_range(from, to)
                .into_iter()
                .filter(|event| kinds.is_empty() || kinds.contains(&event.kind))
                .collect();
            render::print_list(&events, color);
        }
        Command::Add {
            start,
            end,
            kind,
            summary,
        } => {
            let end = end.unwrap_or(start);
            anyhow::ensure!(end >= start, "--end {end} is before --start {start}");
            let event = Event::new(summary.join(" "), start, end, kind);
            println!("added {}", render::event_line(&event, color));
            store.events.push(event);
            store.save()?;
        }
        Command::Bulk { file, kind } => {
            let events = store::parse_bulk(&read_input(&file)?, kind)?;
            for event in &events {
                println!("added {}", render::event_line(event, color));
            }
            let count = events.len();
            store.events.extend(events);
            store.save()?;
            println!(
                "{count} event{} added to {}",
                plural(count),
                store.path().display()
            );
        }
        Command::Import { file, kind } => {
            let text = std::fs::read_to_string(&file)
                .with_context(|| format!("failed to read {}", file.display()))?;
            let events = store::parse_events(&text, kind)?;
            let mut added = 0;
            for event in events {
                // Re-importing the same file must not duplicate events.
                if store.events.iter().any(|known| known.uid == event.uid) {
                    continue;
                }
                println!("imported {}", render::event_line(&event, color));
                store.events.push(event);
                added += 1;
            }
            store.save()?;
            println!(
                "{added} event{} imported into {}",
                plural(added),
                store.path().display()
            );
        }
        Command::Edit {
            uid,
            start,
            end,
            kind,
            summary,
        } => {
            let updated = {
                let event = store.find_mut(&uid)?;
                if let Some(start) = start {
                    event.start = start;
                }
                if let Some(end) = end {
                    event.end = end;
                }
                if let Some(kind) = kind {
                    event.kind = kind;
                }
                if let Some(summary) = summary {
                    event.summary = summary.join(" ");
                }
                event.clone()
            };
            anyhow::ensure!(
                updated.end >= updated.start,
                "end {} is before start {}",
                updated.end,
                updated.start
            );
            println!("updated {}", render::event_line(&updated, color));
            store.save()?;
        }
        Command::Remove { uids } => {
            let removed = uids
                .iter()
                .map(|uid| store.find_mut(uid).map(|event| event.clone()))
                .collect::<Result<Vec<_>>>()?;
            for uid in &uids {
                store.remove(uid)?;
            }
            store.save()?;
            for event in removed {
                println!("removed {}", render::event_line(&event, color));
            }
        }
        Command::Workdays { range } => {
            let (from, to) = range.resolve()?;
            for day in render::days(from, to).filter(|day| store.is_working(*day)) {
                println!("{day}");
            }
        }
        Command::Offdays { range, reason } => {
            let (from, to) = range.resolve()?;
            for day in render::days(from, to).filter(|day| !store.is_working(*day)) {
                if reason {
                    println!("{day}\t{}", store.off_reason(day));
                } else {
                    println!("{day}");
                }
            }
        }
        Command::Path => println!("{}", store.path().display()),
        Command::Calendars => {
            for name in store::list_calendars()? {
                println!("{name}");
            }
        }
    }
    Ok(())
}

const fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

/// Reads a bulk input file, or stdin when the path is `-`.
fn read_input(file: &Path) -> Result<String> {
    if file == Path::new("-") {
        let mut text = String::new();
        std::io::stdin()
            .read_to_string(&mut text)
            .context("failed to read stdin")?;
        return Ok(text);
    }
    std::fs::read_to_string(file).with_context(|| format!("failed to read {}", file.display()))
}
