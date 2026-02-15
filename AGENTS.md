# AGENTS.md

## Project: reap

A macOS CLI tool that watches a process and kills child processes when it dies. Written in Rust.

### Usage

```sh
reap [--watch <pid>] -- <command> [args...]
```

- `--watch <pid>` — PID to monitor (e.g. `--watch $PPID` for sketchybar use case)
- Default (no `--watch`) — watches own parent process via `getppid()`

### Architecture

- Platform abstraction via `Watcher` trait (`src/watcher.rs`)
- macOS: kqueue `EVFILT_PROC` + `NOTE_EXIT` + `EVFILT_SIGNAL` (`src/watcher/macos.rs`)
- Core logic is platform-agnostic (`src/main.rs`)
- Future platforms add a new `watcher/<platform>.rs` implementing the trait

### Technical details

- macOS only for now (Linux has `prctl(PR_SET_PDEATHSIG)`)
- Spawns child in a new process group via `process_group(0)`
- Signals (SIGTERM/SIGINT) blocked with `sigprocmask` and delivered via kqueue `EVFILT_SIGNAL`
- Child spawned before signal blocking so it inherits a clean signal mask
- Graceful shutdown: SIGTERM → 5s poll → SIGKILL
- Exit code: child's code, or `128 + signal_number` for signal death
- crates.io package name: `reap-process` (binary name: `reap`)

### Dependencies

- `argh` — CLI argument parsing
- `nix` — safe Unix syscall wrappers (kqueue, signals, process management)
- `anyhow` — error handling

### Development

- Always run `cargo fmt` before committing.
- Version control: `jj` (Jujutsu), not git. Never run `jj` write commands (e.g. `jj commit`, `jj new`, `jj describe`) unless explicitly instructed. Read-only `jj` commands (e.g. `jj status`, `jj log`, `jj diff`) are fine.
- Commit messages must follow [Conventional Commits](https://www.conventionalcommits.org/) format (e.g. `feat:`, `fix:`, `refactor:`, `test:`, `docs:`, `chore:`). This is required for release-plz.

### Conventions

- All generated output (code, docs, commit messages, comments) must be in English unless explicitly instructed otherwise.
- Code comments should be minimal — only where logic is non-obvious. Comments that merely restate what the immediately following code does are forbidden.
- When instructions should persist across sessions, update this AGENTS.md autonomously.
