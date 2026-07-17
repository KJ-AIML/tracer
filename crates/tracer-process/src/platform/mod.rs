//! Platform process-tree isolation (orphan prevention).
//!
//! - **Windows:** Job Object with `KILL_ON_JOB_CLOSE` (F-W01).
//! - **Unix:** process group (`process_group(0)` + `kill(-pgid, SIGKILL)`).

use crate::error::ProcessError;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::WindowsJobTreeGuard;

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::UnixProcessGroupGuard;

/// Concrete isolation handle used by `ManagedProcess`.
pub enum TreeIsolation {
    /// Isolation disabled (explicit opt-out for tests).
    None,
    /// Windows Job Object with kill-on-close.
    #[cfg(windows)]
    WindowsJob(WindowsJobTreeGuard),
    /// Unix process group.
    #[cfg(unix)]
    UnixGroup(UnixProcessGroupGuard),
}

impl TreeIsolation {
    /// Strategy name for diagnostics / completion evidence.
    pub fn strategy_name(&self) -> &'static str {
        match self {
            Self::None => "null",
            #[cfg(windows)]
            Self::WindowsJob(_) => "windows-job-object-kill-on-close",
            #[cfg(unix)]
            Self::UnixGroup(_) => "unix-process-group",
        }
    }

    /// Prepare command + isolation resources (pre-spawn).
    pub fn prepare(
        command: &mut std::process::Command,
        isolate: bool,
    ) -> Result<Self, ProcessError> {
        if !isolate {
            return Ok(Self::None);
        }
        #[cfg(windows)]
        {
            let _ = command;
            Ok(Self::WindowsJob(WindowsJobTreeGuard::create()?))
        }
        #[cfg(unix)]
        {
            UnixProcessGroupGuard::configure_command(command)?;
            Ok(Self::UnixGroup(UnixProcessGroupGuard::pending()))
        }
        #[cfg(not(any(windows, unix)))]
        {
            let _ = command;
            Ok(Self::None)
        }
    }

    /// Bind after spawn.
    pub fn bind_child(&mut self, child: &std::process::Child) -> Result<(), ProcessError> {
        match self {
            Self::None => Ok(()),
            #[cfg(windows)]
            Self::WindowsJob(job) => job.assign_child(child),
            #[cfg(unix)]
            Self::UnixGroup(pg) => pg.bind_pid(child.id()),
        }
    }

    /// Force-kill the entire tree (best-effort).
    pub fn force_kill_tree(&mut self) -> Result<(), ProcessError> {
        match self {
            Self::None => Ok(()),
            #[cfg(windows)]
            Self::WindowsJob(job) => job.force_kill_tree(),
            #[cfg(unix)]
            Self::UnixGroup(pg) => pg.force_kill_tree(),
        }
    }
}
