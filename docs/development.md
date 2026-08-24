# Development

```bash
make build       # cargo build
make fmt         # cargo fmt --all
make lint        # clippy, warnings denied
make test        # cargo test
make doc         # rustdoc, warnings denied
make check       # all of the above, same as CI
```

`mise` provides the toolchain and the same tasks (`mise run check`).

## Layout

| File            | Contents                                              |
| --------------- | ----------------------------------------------------- |
| `src/main.rs`   | command dispatch                                      |
| `src/cli.rs`    | clap definitions and range resolution                 |
| `src/store.rs`  | events, kinds, iCalendar read/write, working-day rules |
| `src/render.rs` | terminal month/year grids and tables                  |

## Release

CI runs quality checks, an MSRV build, macOS builds for both Apple targets and
`cargo audit` on every pull request.

Pushes to `master` run `release-plz`, which bumps the version from
Conventional Commits and publishes to crates.io. The release then gets a Linux
tarball, a `.deb`, an `.rpm`, macOS tarballs for `aarch64-apple-darwin` and
`x86_64-apple-darwin`, checksums and build provenance attestations. Finally the
`.deb` and `.rpm` of the last ten releases are published as APT and RPM
repositories on GitHub Pages.
