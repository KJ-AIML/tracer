//! Windows Job Object isolation with kill-on-close (F-W01 / F-P11).

use std::process::Child;
use std::ptr;

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};
use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE};

use crate::error::ProcessError;

/// Owns a Job Object handle. Dropping the handle kills remaining job members
/// when `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` is set.
pub struct WindowsJobTreeGuard {
    handle: HANDLE,
    assigned: bool,
}

// SAFETY: HANDLE is used only through this guard's exclusive ownership model;
// ManagedProcess does not share the guard across threads concurrently.
unsafe impl Send for WindowsJobTreeGuard {}

impl WindowsJobTreeGuard {
    /// Create a job with kill-on-close.
    pub fn create() -> Result<Self, ProcessError> {
        // SAFETY: null name/attrs → anonymous job object.
        let handle = unsafe { CreateJobObjectW(ptr::null(), ptr::null()) };
        if handle.is_null() || handle == INVALID_HANDLE_VALUE {
            return Err(ProcessError::internal(format!(
                "CreateJobObjectW failed: {}",
                std::io::Error::last_os_error()
            )));
        }

        let mut info = unsafe { std::mem::zeroed::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() };
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

        // SAFETY: handle is a valid job; size matches the information class.
        let ok = unsafe {
            SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };
        if ok == 0 {
            unsafe {
                CloseHandle(handle);
            }
            return Err(ProcessError::internal(format!(
                "SetInformationJobObject(KILL_ON_JOB_CLOSE) failed: {}",
                std::io::Error::last_os_error()
            )));
        }

        Ok(Self {
            handle,
            assigned: false,
        })
    }

    /// Assign a spawned child to this job by pid.
    pub fn assign_child(&mut self, child: &Child) -> Result<(), ProcessError> {
        self.assign_pid(child.id())
    }

    /// Assign by OS process id.
    pub fn assign_pid(&mut self, pid: u32) -> Result<(), ProcessError> {
        // Rights required for job assignment: TERMINATE | SET_QUOTA.
        let access = PROCESS_TERMINATE | PROCESS_SET_QUOTA;
        // SAFETY: OpenProcess with known pid; handle closed on all paths.
        let process = unsafe { OpenProcess(access, 0, pid) };
        if process.is_null() || process == INVALID_HANDLE_VALUE {
            return Err(ProcessError::internal(format!(
                "OpenProcess({pid}) for job assignment failed: {}",
                std::io::Error::last_os_error()
            )));
        }

        // SAFETY: both handles valid for AssignProcessToJobObject.
        let ok = unsafe { AssignProcessToJobObject(self.handle, process) };
        let assign_err = if ok == 0 {
            Some(std::io::Error::last_os_error())
        } else {
            None
        };

        unsafe {
            CloseHandle(process);
        }

        if let Some(err) = assign_err {
            return Err(ProcessError::internal(format!(
                "AssignProcessToJobObject failed: {err}"
            )));
        }

        self.assigned = true;
        Ok(())
    }

    /// Terminate every process currently in the job.
    pub fn force_kill_tree(&mut self) -> Result<(), ProcessError> {
        // SAFETY: valid job handle.
        let ok = unsafe { TerminateJobObject(self.handle, 1) };
        if ok == 0 {
            if !self.assigned {
                return Ok(());
            }
            let err = std::io::Error::last_os_error();
            // Already empty / closed races are acceptable on force paths.
            if err.raw_os_error() == Some(5) {
                // ERROR_ACCESS_DENIED can appear if processes already exiting.
                return Ok(());
            }
            return Err(ProcessError::internal(format!(
                "TerminateJobObject failed: {err}"
            )));
        }
        Ok(())
    }
}

impl Drop for WindowsJobTreeGuard {
    fn drop(&mut self) {
        if !self.handle.is_null() && self.handle != INVALID_HANDLE_VALUE {
            // CloseHandle on a kill-on-close job terminates remaining members
            // (orphan prevention when the parent app crashes or drops the handle).
            // SAFETY: exclusive ownership of handle.
            unsafe {
                CloseHandle(self.handle);
            }
            self.handle = ptr::null_mut();
        }
    }
}
