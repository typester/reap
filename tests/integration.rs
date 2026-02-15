use std::process::Command;
use std::time::Duration;

use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;

fn reap_bin() -> String {
    env!("CARGO_BIN_EXE_reap").to_string()
}

#[test]
fn basic_run() {
    let output = Command::new(reap_bin())
        .args(["echo", "hello"])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "hello");
}

#[test]
fn child_exit_code() {
    let status = Command::new(reap_bin()).arg("false").status().unwrap();

    assert_eq!(status.code(), Some(1));
}

#[test]
fn command_not_found() {
    let output = Command::new(reap_bin())
        .arg("nonexistent_cmd_that_does_not_exist")
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failed to spawn"), "stderr: {stderr}");
}

#[test]
fn watched_pid_already_dead() {
    // Spawn and immediately kill a process to get a guaranteed-dead PID
    let mut proc = Command::new("sleep").arg("999").spawn().unwrap();
    let dead_pid = proc.id();
    proc.kill().unwrap();
    proc.wait().unwrap();

    let output = Command::new(reap_bin())
        .args(["--watch", &dead_pid.to_string(), "--", "sleep", "60"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("watched process does not exist"),
        "stderr: {stderr}"
    );
}

#[test]
fn watched_pid_death_kills_child() {
    // Spawn a process to watch
    let mut watched = Command::new("sleep").arg("999").spawn().unwrap();
    let watched_pid = watched.id();

    // Start reap watching that process
    let mut reap = Command::new(reap_bin())
        .args(["--watch", &watched_pid.to_string(), "--", "sleep", "999"])
        .spawn()
        .unwrap();

    std::thread::sleep(Duration::from_millis(500));

    // Kill the watched process
    watched.kill().unwrap();
    watched.wait().unwrap();

    // reap should exit within a few seconds
    let status = wait_with_timeout(&mut reap, Duration::from_secs(10))
        .expect("reap did not exit after watched process died");

    assert!(!status.success());
}

#[test]
fn signal_forwarding_sigterm() {
    let mut reap = Command::new(reap_bin())
        .args(["sleep", "60"])
        .spawn()
        .unwrap();

    std::thread::sleep(Duration::from_millis(500));

    let reap_pid = Pid::from_raw(reap.id() as i32);
    kill(reap_pid, Signal::SIGTERM).unwrap();

    let status = wait_with_timeout(&mut reap, Duration::from_secs(10))
        .expect("reap did not exit after SIGTERM");

    // 143 = 128 + 15 (SIGTERM)
    assert_eq!(status.code(), Some(143));
}

#[test]
fn signal_forwarding_sigint() {
    let mut reap = Command::new(reap_bin())
        .args(["sleep", "60"])
        .spawn()
        .unwrap();

    std::thread::sleep(Duration::from_millis(500));

    let reap_pid = Pid::from_raw(reap.id() as i32);
    kill(reap_pid, Signal::SIGINT).unwrap();

    let status = wait_with_timeout(&mut reap, Duration::from_secs(10))
        .expect("reap did not exit after SIGINT");

    // 130 = 128 + 2 (SIGINT)
    assert_eq!(status.code(), Some(130));
}

fn wait_with_timeout(
    child: &mut std::process::Child,
    timeout: Duration,
) -> Option<std::process::ExitStatus> {
    let start = std::time::Instant::now();
    loop {
        match child.try_wait().unwrap() {
            Some(status) => return Some(status),
            None => {
                if start.elapsed() >= timeout {
                    child.kill().ok();
                    return None;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
}
