//! Unix process-group isolation for tree kill (F-P11 / F-U01x).

use std::os::unix::process::CommandExt;
use std::process::Command;

use crate::error::ProcessError;

/// Process-group based tree kill.
pub struct UnixProcessGroupGuard {
    /// Leader pid (also the process group id when started with `process_group(0)`).
    pgid: Option<u32>,
}

impl UnixProcessGroupGuard {
    /// Configure `command` to start a new process group (pgid = child pid).
    pub fn configure_command(command: &mut Command) -> Result<(), ProcessError> {
        // `process_group(0)` → child becomes leader of a new group.
        command.process_group(0);
        Ok(())
    }

    /// Create a pending guard (bind after spawn).
    pub fn pending() -> Self {
        Self { pgid: None }
    }

    /// Record the leader pid after spawn.
    pub fn bind_pid(&mut self, pid: u32) -> Result<(), ProcessError> {
        self.pgid = Some(pid);
        Ok(())
    }

    /// Send SIGKILL to the entire process group.
    pub fn force_kill_tree(&mut self) -> Result<(), ProcessError> {
        let Some(pgid) = self.pgid else {
            return Ok(());
        };
        // Negative pgid => kill process group.
        // SAFETY: kill(2) with -pgid targets the group; ESRCH is benign.
        let rc = unsafe { libc::kill(-(pgid as i32), libc::SIGKILL) };
        if rc != 0 {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::ESRCH) {
                return Ok(());
            }
            return Err(ProcessError::internal(format!(
                "kill(-{pgid}, SIGKILL) failed: {err}"
            )));
        }
        Ok(())
    }
}
