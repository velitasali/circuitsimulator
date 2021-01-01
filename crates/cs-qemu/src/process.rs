//! Child `qemu-system-*` process. SIGSTOP/SIGCONT keep it in lockstep with a
//! paused circuit (see `QemuDevice::stopQemuProcess`).

use crate::{Error, Result};
use std::path::Path;
use std::process::{Child, Command, Stdio};

pub struct QemuProcess {
    child: Option<Child>,
    stopped: bool,
    /// When a GDB stub is attached, do not SIGSTOP — the debugger owns pause.
    pub gdb: bool,
}

impl QemuProcess {
    pub fn spawn(executable: &Path, args: &[String]) -> Result<Self> {
        if !executable.is_file() {
            return Err(Error::Process(format!(
                "emulator executable not found: {}",
                executable.display()
            )));
        }
        let child = Command::new(executable)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| {
                Error::Process(format!("could not spawn {}: {e}", executable.display()))
            })?;
        Ok(Self {
            child: Some(child),
            stopped: false,
            gdb: false,
        })
    }

    pub fn is_running(&mut self) -> bool {
        let Some(child) = self.child.as_mut() else {
            return false;
        };
        match child.try_wait() {
            Ok(None) => true,
            Ok(Some(_)) => {
                self.child = None;
                self.stopped = false;
                false
            }
            Err(_) => false,
        }
    }

    /// SIGSTOP. No-op if GDB is attached, already stopped, or not running.
    pub fn stop(&mut self) {
        if self.stopped || self.gdb || !self.is_running() {
            return;
        }
        #[cfg(unix)]
        if let Some(child) = &self.child {
            let pid = child.id() as libc::pid_t;
            if unsafe { libc::kill(pid, libc::SIGSTOP) } == 0 {
                self.stopped = true;
            }
        }
    }

    pub fn resume(&mut self) {
        if !self.stopped {
            return;
        }
        #[cfg(unix)]
        if let Some(child) = &self.child {
            let pid = child.id() as libc::pid_t;
            if unsafe { libc::kill(pid, libc::SIGCONT) } == 0 {
                self.stopped = false;
            }
        }
        #[cfg(not(unix))]
        {
            self.stopped = false;
        }
    }

    pub fn kill(&mut self) {
        if let Some(mut child) = self.child.take() {
            #[cfg(unix)]
            {
                let pid = child.id() as libc::pid_t;
                unsafe { libc::kill(pid, libc::SIGKILL) };
            }
            let _ = child.kill();
            let _ = child.wait();
        }
        self.stopped = false;
    }

    pub fn stopped(&self) -> bool {
        self.stopped
    }
}

impl Drop for QemuProcess {
    fn drop(&mut self) {
        self.kill();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_binary() {
        let err = match QemuProcess::spawn(Path::new("/no/such/qemu-system-xtensa"), &[]) {
            Ok(_) => panic!("expected missing binary"),
            Err(e) => e,
        };
        let msg = err.to_string();
        assert!(msg.contains("not found"), "{msg}");
    }

    #[cfg(unix)]
    #[test]
    fn spawn_sleep_stop_resume_kill() {
        let mut p = QemuProcess::spawn(Path::new("/bin/sleep"), &["8".into()]).expect("sleep");
        assert!(p.is_running());
        p.stop();
        assert!(p.stopped());
        assert!(p.is_running());
        p.resume();
        assert!(!p.stopped());
        p.kill();
        assert!(!p.is_running());
    }
}
