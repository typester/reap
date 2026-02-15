# reap

Watch a process and kill child processes when it dies.

On macOS, when a parent process exits, its child processes are reparented to `launchd` and keep running. `reap` solves this by monitoring a specified process via kqueue and terminating the child process group when it exits.

## Usage

```sh
reap [--watch <pid>] -- <command> [args...]
```

By default, `reap` watches its own parent process. Use `--watch <pid>` to watch an arbitrary process.

### Example: sketchybar

In sketchybar's config script (`sketchybarrc`), background processes outlive sketchybar because the shell script exits immediately. Use `$PPID` (which is sketchybar's PID from the script's perspective) to watch sketchybar:

```sh
reap --watch $PPID -- ./plugin.sh &
```

When sketchybar exits, `reap` kills `plugin.sh` and its descendants.

## How it works

1. Spawns the child command in a new process group
2. Monitors the watched PID using kqueue `EVFILT_PROC` + `NOTE_EXIT` (zero polling overhead)
3. Forwards `SIGTERM`/`SIGINT` to the child process group
4. When the watched process exits: sends `SIGTERM` to the child group, waits up to 5 seconds, then escalates to `SIGKILL`
5. Propagates the child's exit code (or `128 + signal` if killed by a signal)

## Installation

```sh
cargo install reap-process
```

## Building from source

```sh
git clone https://github.com/typester/reap.git
cd reap
cargo build --release
```

## Requirements

- macOS (uses kqueue, which has no Linux equivalent for this purpose)

## License

MIT
