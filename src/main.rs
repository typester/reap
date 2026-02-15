mod watcher;

use std::os::unix::process::CommandExt;
use std::process::Command;

use anyhow::{Context, bail};
use argh::FromArgs;
use nix::sys::signal::{SigSet, SigmaskHow, Signal, killpg, sigprocmask};
use nix::sys::wait::{WaitPidFlag, WaitStatus, waitpid};
use nix::unistd::{Pid, getppid};

use watcher::{Event, PlatformWatcher, Watcher};

/// Watch a process and kill child processes when it dies.
#[derive(FromArgs)]
struct Args {
    /// PID to watch (default: parent process)
    #[argh(option)]
    watch: Option<i32>,

    /// command to run
    #[argh(positional, greedy)]
    command: Vec<String>,
}

fn main() {
    let code = match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("reap: {e:#}");
            1
        }
    };
    std::process::exit(code);
}

fn run() -> anyhow::Result<i32> {
    let args: Args = argh::from_env();

    if args.command.is_empty() {
        bail!("usage: reap [--watch <pid>] -- <command> [args...]");
    }

    let watch_pid = match args.watch {
        Some(pid) => Pid::from_raw(pid),
        None => getppid(),
    };

    // Spawn child before blocking signals so it inherits a clean signal mask
    let child = Command::new(&args.command[0])
        .args(&args.command[1..])
        .process_group(0)
        .spawn()
        .with_context(|| format!("failed to spawn '{}'", args.command[0]))?;

    let child_pid = Pid::from_raw(child.id() as i32);

    // We manage the child via waitpid directly, not through std::process::Child
    drop(child);

    let result = run_event_loop(watch_pid, child_pid);
    if result.is_err() {
        kill_child_group(child_pid);
        wait_for_child(child_pid);
    }
    result
}

fn run_event_loop(watch_pid: Pid, child_pid: Pid) -> anyhow::Result<i32> {
    let mut mask = SigSet::empty();
    mask.add(Signal::SIGTERM);
    mask.add(Signal::SIGINT);
    sigprocmask(SigmaskHow::SIG_BLOCK, Some(&mask), None).context("failed to block signals")?;

    let mut watcher = PlatformWatcher::new(watch_pid, child_pid)?;

    loop {
        match watcher.wait()? {
            Event::WatchedPidExited => {
                kill_child_group(child_pid);
                return Ok(wait_for_child(child_pid));
            }
            Event::ChildExited => {
                return Ok(wait_for_child(child_pid));
            }
            Event::Signal(sig) => {
                let _ = killpg(child_pid, sig);
            }
        }
    }
}

fn kill_child_group(child_pid: Pid) {
    let _ = killpg(child_pid, Signal::SIGTERM);

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);

    loop {
        match waitpid(child_pid, Some(WaitPidFlag::WNOHANG)) {
            Ok(WaitStatus::StillAlive) => {
                if std::time::Instant::now() >= deadline {
                    let _ = killpg(child_pid, Signal::SIGKILL);
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            _ => return,
        }
    }
}

fn wait_for_child(child_pid: Pid) -> i32 {
    match waitpid(child_pid, None) {
        Ok(WaitStatus::Exited(_, code)) => code,
        Ok(WaitStatus::Signaled(_, sig, _)) => 128 + sig as i32,
        _ => 1,
    }
}
