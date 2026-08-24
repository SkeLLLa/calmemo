# Getting started

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

macOS and plain Linux tarballs (`aarch64-apple-darwin`,
`x86_64-apple-darwin`, `x86_64-unknown-linux-gnu`) are attached to every
[release](https://github.com/SkeLLLa/calmemo/releases).

## Look at the calendar

```bash
calmemo             # current month, colored, with the event legend
calmemo cal -m 2026-08
calmemo cal -y 2026 # twelve months in three columns
calmemo cal -f 2026-11-15 -t 2027-01-10   # every month the range touches
```

## Add events

The kind is the type of day. Leave `-k` out and you get a plain `dayoff`.

```bash
calmemo add -s 2026-12-24 Moving day             # dayoff
calmemo add -s 2026-12-25 -k holiday Christmas
calmemo add -s 2026-08-03 -e 2026-08-14 -k vacation Summer break
calmemo add -s 2026-09-07 -k sick Flu
calmemo add -s 2026-12-26 -k workday Catch-up Saturday
```

Bulk input uses `start;end;kind;summary` lines, `end` and `kind` may be empty,
`#` starts a comment:

```bash
calmemo bulk holidays.txt
printf '2027-01-01;;holiday;New Year\n2027-05-01;2027-05-02;;Long weekend\n' | calmemo bulk
```

Public-holiday feeds can be merged directly:

```bash
calmemo import ua-holidays.ics -k holiday
```

## Several calendars

```bash
calmemo -n work add -s 2026-12-25 -k holiday Christmas
calmemo -n personal ls
calmemo calendars          # names that exist
calmemo -n work path       # ~/.config/calmemo/work.ics
```

Without `-n` everything goes to the `default` calendar.

## Change and delete

Events are addressed by any unique prefix of their UID (as printed by
`calmemo ls`):

```bash
calmemo ls
calmemo edit 3f2a1c -e 2026-08-16 --summary Summer break, extended
calmemo rm 3f2a1c
```

## Report working days

Plain, one ISO date per line, ready for `wc -l` or a time sheet. The default
range is the whole current month, whether or not it has ended:

```bash
calmemo workdays
calmemo offdays --reason
calmemo workdays -m 2026-08
calmemo workdays -y 2026 | wc -l
calmemo workdays -f 2026-08-15          # to the end of August
calmemo workdays -f 2026-08-01 -t 2026-09-15
```
