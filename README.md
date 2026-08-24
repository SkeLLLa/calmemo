# calmemo

Local-first calendar CLI: iCalendar files with public holidays, vacations
and sick leaves, a colored terminal calendar, and a plain list of working days
for your time sheet.

- **Local storage** — `~/.config/calmemo/<name>.ics`, plain RFC 5545, no server
- **Fancy view** — colored month, range or whole-year grid with the legend
- **Script-friendly** — `calmemo workdays` prints one ISO date per line
- **Bulk friendly** — `calmemo bulk` for `;`-separated lines, `calmemo import`
  for existing `.ics` holiday feeds
- **Sensible defaults** — every command works on the whole current month until
  you say otherwise

## Install

```bash
cargo install calmemo
```

Fedora / RHEL:

```bash
sudo tee /etc/yum.repos.d/calmemo.repo >/dev/null <<'REPO'
[calmemo]
name=calmemo
baseurl=https://skellla.github.io/calmemo/rpm/x86_64
enabled=1
gpgcheck=0
repo_gpgcheck=0
REPO
sudo dnf install calmemo
```

Debian / Ubuntu:

```bash
echo 'deb [trusted=yes] https://skellla.github.io/calmemo/deb stable main' \
  | sudo tee /etc/apt/sources.list.d/calmemo.list
sudo apt update && sudo apt install calmemo
```

Or grab a Linux / macOS tarball, a `.deb` or an `.rpm` from the
[releases](https://github.com/SkeLLLa/calmemo/releases).

## Usage

```bash
calmemo                                              # current month
calmemo cal -m 2026-12                               # a specific month
calmemo cal -y 2026                                  # the whole year
calmemo cal -f 2026-11-15 -t 2027-01-10              # every month in the range

calmemo add -s 2026-12-24 Moving day                 # kind defaults to dayoff
calmemo add -s 2026-12-25 -k holiday Christmas
calmemo add -s 2026-12-28 -e 2026-12-31 -k vacation Winter break
calmemo add -s 2026-12-19 -k workday Release freeze  # working Saturday

calmemo ls                                           # table with UIDs
calmemo edit a7fdc8 -k note                          # UID prefix is enough
calmemo rm a7fdc8

calmemo workdays                                     # one ISO date per line
calmemo offdays --reason
calmemo workdays -y 2026 | wc -l
calmemo path                                         # where the file lives
```

Ranges default to the whole current month, past days included. `--month`,
`--year`, `--from`/`--to` override that; a lone `--from` runs to the end of its
month, a lone `--to` starts at the beginning of its month.

```text
$ calmemo cal -m 2026-12
    December 2026
Mo Tu We Th Fr Sa Su
    1  2  3  4  5  6
 7  8  9 10 11 12 13
14 15 16 17 18 19 20
21 22 23 24 25 26 27
28 29 30 31

  ee8d93df dayoff    2026-12-24                      Moving day
  2f4ea95f holiday   2026-12-25                      Christmas
  a45b0b61 vacation  2026-12-28 → 2026-12-31 (4d)    Winter break

3 events, 17 working days, 14 days off
key: holiday vacation sick dayoff workday note weekend
```

## Several calendars

Each calendar is a separate file. `--name` picks one, `default` is used when
you do not:

```bash
calmemo -n work add -s 2026-12-25 -k holiday Christmas
calmemo -n personal ls
calmemo calendars      # names that exist
calmemo -n work path   # ~/.config/calmemo/work.ics
```

### Bulk input

`start;end;kind;summary`, where `end` and `kind` may be empty and `#` starts a
comment. A three-field line is `start;kind;summary`.

```bash
calmemo bulk holidays.txt
printf '2027-01-01;;holiday;New Year\n2027-05-01;2027-05-04;vacation;May\n' | calmemo bulk
calmemo import ua-public-holidays.ics -k holiday   # any .ics feed
```

## Event kinds

`--kind` says what type of day an event is. Each kind, and the weekend, has its
own color in the calendar.

| Kind       | Effect on working days          | Color        |
| ---------- | ------------------------------- | ------------ |
| `holiday`  | day off, even on a weekday      | red          |
| `vacation` | day off                         | cyan         |
| `sick`     | day off                         | magenta      |
| `dayoff`   | day off, **the default**        | yellow       |
| `workday`  | working day, overrides weekends | green        |
| `note`     | no effect, only shown           | blue         |
| —          | weekend                         | bright black |

Saturdays and Sundays are off unless a `workday` event says otherwise. Weeks
start on Monday. Today is shown inverted.

## Development

`make check` runs the same suite as CI (fmt, clippy, tests, docs). See
[docs/development.md](docs/development.md).

## License

MIT
