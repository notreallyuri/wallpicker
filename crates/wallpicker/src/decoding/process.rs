use std::io;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, ExitStatus};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct State {
    child: Option<Child>,
    stopped: bool,
}

#[derive(Default)]
pub(super) struct Process {
    state: Mutex<State>,
}

impl Process {
    pub(super) fn start(&self, command: &mut Command) -> io::Result<()> {
        let mut state = self.state.lock().unwrap();
        if state.stopped {
            return Err(io::Error::new(io::ErrorKind::Interrupted, "decoder cancelled"));
        }
        state.child = Some(command.spawn()?);
        Ok(())
    }

    pub(super) fn id(&self) -> Option<u32> {
        self.state.lock().unwrap().child.as_ref().map(Child::id)
    }

    pub(super) fn stop(&self) -> Option<ExitStatus> {
        let mut state = self.state.lock().unwrap();
        state.stopped = true;
        let mut child = state.child.take()?;
        // try_wait reaps an already-exited child, preserving its failure status.
        if let Ok(Some(status)) = child.try_wait() {
            return Some(status);
        }
        let _ = child.kill();
        child.wait().ok()
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Also runs on a pump-thread panic, before the thread releases its reservation.
pub(super) struct Reaper(pub Arc<Process>);
impl Drop for Reaper {
    fn drop(&mut self) {
        self.0.stop();
    }
}

pub(super) fn constrain(command: &mut Command, bytes: u64) {
    let parent = std::process::id() as libc::pid_t;
    // SAFETY: only async-signal-safe system calls, fixed stack values and errno
    // access occur after fork. No allocator, locks or formatting in this closure.
    unsafe {
        command.pre_exec(move || {
            let memory = libc::rlimit {
                rlim_cur: bytes as libc::rlim_t,
                rlim_max: bytes as libc::rlim_t,
            };
            let core = libc::rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            if libc::setrlimit(libc::RLIMIT_AS, &memory) != 0
                || libc::setrlimit(libc::RLIMIT_CORE, &core) != 0
                || libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) != 0
            {
                return Err(io::Error::last_os_error());
            }
            // Close the race where the parent died before installing PDEATHSIG.
            if libc::getppid() != parent {
                libc::_exit(1);
            }
            Ok(())
        });
    }
}
