use nix::sys::signal::Signal;

#[derive(Debug)]
pub enum Event {
    WatchedPidExited,
    ChildExited,
    Signal(Signal),
}

pub trait Watcher {
    fn new(watch_pid: nix::unistd::Pid, child_pid: nix::unistd::Pid) -> anyhow::Result<Self>
    where
        Self: Sized;

    fn wait(&mut self) -> anyhow::Result<Event>;
}

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub use macos::KqueueWatcher as PlatformWatcher;
