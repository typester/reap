use anyhow::{Context, bail};
use nix::libc::uintptr_t;
use nix::sys::event::{EvFlags, EventFilter, FilterFlag, KEvent, Kqueue};
use nix::sys::signal::Signal;
use nix::unistd::Pid;

use super::{Event, Watcher};

pub struct KqueueWatcher {
    kq: Kqueue,
    watch_pid: Pid,
    child_pid: Pid,
}

impl Watcher for KqueueWatcher {
    fn new(watch_pid: Pid, child_pid: Pid) -> anyhow::Result<Self> {
        let kq = Kqueue::new().context("failed to create kqueue")?;

        // Register watched PID separately to detect ESRCH (already dead)
        let parent_event = [KEvent::new(
            watch_pid.as_raw() as uintptr_t,
            EventFilter::EVFILT_PROC,
            EvFlags::EV_ADD | EvFlags::EV_ONESHOT,
            FilterFlag::NOTE_EXIT,
            0,
            0,
        )];
        kq.kevent(&parent_event, &mut [], None)
            .context("watched process does not exist")?;

        let events = [
            KEvent::new(
                child_pid.as_raw() as uintptr_t,
                EventFilter::EVFILT_PROC,
                EvFlags::EV_ADD | EvFlags::EV_ONESHOT,
                FilterFlag::NOTE_EXIT,
                0,
                0,
            ),
            KEvent::new(
                Signal::SIGTERM as uintptr_t,
                EventFilter::EVFILT_SIGNAL,
                EvFlags::EV_ADD,
                FilterFlag::empty(),
                0,
                0,
            ),
            KEvent::new(
                Signal::SIGINT as uintptr_t,
                EventFilter::EVFILT_SIGNAL,
                EvFlags::EV_ADD,
                FilterFlag::empty(),
                0,
                0,
            ),
        ];
        kq.kevent(&events, &mut [], None)
            .context("failed to register kqueue events")?;

        Ok(Self {
            kq,
            watch_pid,
            child_pid,
        })
    }

    fn wait(&mut self) -> anyhow::Result<Event> {
        let mut events = [KEvent::new(
            0,
            EventFilter::EVFILT_PROC,
            EvFlags::empty(),
            FilterFlag::empty(),
            0,
            0,
        ); 4];

        loop {
            let n = self
                .kq
                .kevent(&[], &mut events, None)
                .context("kevent wait failed")?;

            for event in &events[..n] {
                let filter = event.filter().context("unknown kqueue filter")?;

                match filter {
                    EventFilter::EVFILT_PROC => {
                        let pid = Pid::from_raw(event.ident() as i32);
                        if pid == self.watch_pid {
                            return Ok(Event::WatchedPidExited);
                        }
                        if pid == self.child_pid {
                            return Ok(Event::ChildExited);
                        }
                    }
                    EventFilter::EVFILT_SIGNAL => {
                        let sig = Signal::try_from(event.ident() as i32);
                        match sig {
                            Ok(s) => return Ok(Event::Signal(s)),
                            Err(_) => bail!("unknown signal: {}", event.ident()),
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
