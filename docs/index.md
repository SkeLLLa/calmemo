# calmemo

`calmemo` is a local-first calendar CLI. It keeps a single iCalendar file with
public holidays, vacations, sick leaves and notes, renders it in the terminal
and answers the one question a time sheet needs: which days are working days?

## Storage

Each calendar is one file: `$XDG_CONFIG_HOME/calmemo/<name>.ics`
(`~/.config/calmemo/<name>.ics`), where `<name>` comes from `--name` and
defaults to `default`. `--calendar <FILE>` overrides the path entirely.
The format is plain [RFC 5545](https://www.rfc-editor.org/rfc/rfc5545)
iCalendar, so any calendar application can read it, and public-holiday `.ics`
feeds can be imported as-is.

Every event is an all-day `VEVENT` with an inclusive date range and a
`CATEGORIES` property carrying its kind:

| Kind       | Effect on working days          | Color        |
| ---------- | ------------------------------- | ------------ |
| `holiday`  | day off, even on a weekday      | red          |
| `vacation` | day off                         | cyan         |
| `sick`     | day off                         | magenta      |
| `dayoff`   | day off, the default kind       | yellow       |
| `workday`  | working day, overrides weekends | green        |
| `note`     | no effect, shown on the grid    | blue         |
| —          | weekend                         | bright black |

Weekends (Saturday, Sunday) are off unless a `workday` event overrides them.
An event whose type is not given is a `dayoff`.

## Ranges

Every command that takes a range accepts `--month YYYY-MM`, `--year YYYY` or
`--from`/`--to`, and defaults to the **whole current month**, including days
that already passed and days after today. A lone `--from` runs to the end of
its month, a lone `--to` starts at the beginning of its month.

## Modules

- [`getting_started`](getting_started/index.html) — install and daily usage
- [`development`](development/index.html) — build, test and release
