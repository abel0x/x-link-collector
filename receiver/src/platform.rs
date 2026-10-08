// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 abel0x <https://github.com/abel0x>

//! What this daemon needs from the operating system: a way to wait for a
//! connection without spinning, a way to hear about shutdown, and -- since the
//! panel runs the Python tools -- a way to stop a child and everything it
//! started, plus opening a folder in the file manager.
//!
//! Unix gets `poll(2)`, `signal(2)` and `kill(2)`, declared here rather than
//! pulled in as a dependency. Windows gets a short sleep between non-blocking
//! accepts and a console control handler. That asymmetry is deliberate: the
//! Windows build is cross-checked but cannot be run from the machine this was
//! written on, so it uses the version that has no way to be subtly wrong. At a
//! few requests a minute, a 25 ms tick costs nothing measurable either way.

use std::ffi::OsStr;
use std::io;
use std::net::TcpListener;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

/// Set once a shutdown signal arrives; the accept loop checks it between waits.
pub static SHUTDOWN: AtomicBool = AtomicBool::new(false);

pub fn shutting_down() -> bool {
    SHUTDOWN.load(Ordering::Relaxed)
}

/// Stop the accept loop, as a signal would: the panel's Quit button.
pub fn request_shutdown() {
    SHUTDOWN.store(true, Ordering::SeqCst);
}

#[cfg(unix)]
mod imp {
    use super::*;
    use std::os::unix::io::AsRawFd;

    use std::os::unix::process::CommandExt;

    const SIGHUP: i32 = 1;
    const SIGINT: i32 = 2;
    const SIGKILL: i32 = 9;
    const SIGTERM: i32 = 15;
    const POLLIN: i16 = 0x001;

    #[repr(C)]
    struct PollFd {
        fd: i32,
        events: i16,
        revents: i16,
    }

    extern "C" {
        fn signal(signum: i32, handler: usize) -> usize;
        fn poll(fds: *mut PollFd, nfds: usize, timeout: i32) -> i32;
        fn kill(pid: i32, sig: i32) -> i32;
    }

    /// A job gets a process group of its own, so the daemon can signal the
    /// whole tree under it -- x-download, its yt-dlp, their ffmpeg.
    pub fn detach(cmd: &mut Command) {
        cmd.process_group(0);
    }

    /// Ctrl-C for a job: SIGINT to its group, exactly what a terminal sends.
    /// x-download saves its state on the way out.
    pub fn interrupt_tree(pid: u32) {
        unsafe {
            kill(-(pid as i32), SIGINT);
        }
    }

    pub fn kill_tree(pid: u32) {
        unsafe {
            kill(-(pid as i32), SIGKILL);
        }
    }

    /// A file manager here runs a program inside a terminal, if at all; there
    /// is nothing to tell apart.
    pub fn double_clicked() -> bool {
        false
    }

    pub fn detach_console() {}

    /// `make service` installs a systemd or launchd unit for this; the panel
    /// does not offer it on these systems.
    pub fn autostart() -> Option<bool> {
        None
    }

    pub fn set_autostart(_on: bool) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "use make service",
        ))
    }

    extern "C" fn on_signal(_signum: i32) {
        // Setting an atomic is the one thing that is always signal-safe.
        SHUTDOWN.store(true, Ordering::SeqCst);
    }

    pub fn install_shutdown_handlers() {
        let handler = on_signal as extern "C" fn(i32) as usize;
        unsafe {
            signal(SIGINT, handler);
            signal(SIGTERM, handler);
            signal(SIGHUP, handler);
        }
    }

    /// Block until the listener has a pending connection, the timeout expires,
    /// or a signal interrupts the wait.
    pub fn wait_for_connection(listener: &TcpListener, timeout_ms: i32) -> bool {
        let mut pfd = PollFd {
            fd: listener.as_raw_fd(),
            events: POLLIN,
            revents: 0,
        };
        let rc = unsafe { poll(&mut pfd, 1, timeout_ms) };
        rc > 0 && (pfd.revents & POLLIN) != 0
    }
}

#[cfg(windows)]
mod imp {
    use super::*;
    use std::os::windows::process::CommandExt;
    use std::time::Duration;

    /// No console window flashing up for every job.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    /// Ctrl-C, Ctrl-Break, console closed, logoff, shutdown.
    const HANDLED_EVENTS: [u32; 5] = [0, 1, 2, 5, 6];
    const TRUE: i32 = 1;

    type HandlerRoutine = unsafe extern "system" fn(u32) -> i32;

    extern "system" {
        fn SetConsoleCtrlHandler(handler: Option<HandlerRoutine>, add: i32) -> i32;
        fn GetConsoleProcessList(list: *mut u32, count: u32) -> u32;
        fn FreeConsole() -> i32;
    }

    /// Where Windows keeps what starts at logon, for this user only.
    const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
    const RUN_VALUE: &str = "x-link-receiver";

    fn reg(args: &[&str]) -> io::Result<std::process::ExitStatus> {
        Command::new("reg")
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .status()
    }

    /// Let go of the console window Explorer opened for us. The panel is the
    /// way in from here on, and the log goes to a file.
    pub fn detach_console() {
        unsafe {
            FreeConsole();
        }
    }

    /// Whether this receiver starts when the user logs on.
    pub fn autostart() -> Option<bool> {
        Some(reg(&["query", RUN_KEY, "/v", RUN_VALUE]).map_or(false, |s| s.success()))
    }

    /// Start at logon, without a browser tab each time, or stop doing so.
    pub fn set_autostart(on: bool) -> io::Result<()> {
        let status = if on {
            let exe = std::env::current_exe()?;
            let command = format!("\"{}\" --no-open", exe.display());
            reg(&[
                "add", RUN_KEY, "/v", RUN_VALUE, "/t", "REG_SZ", "/d", &command, "/f",
            ])?
        } else {
            reg(&["delete", RUN_KEY, "/v", RUN_VALUE, "/f"])?
        };
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "reg.exe refused"))
        }
    }

    unsafe extern "system" fn on_console_event(event: u32) -> i32 {
        if HANDLED_EVENTS.contains(&event) {
            SHUTDOWN.store(true, Ordering::SeqCst);
            return TRUE; // handled; do not let the default handler kill us
        }
        0
    }

    pub fn install_shutdown_handlers() {
        // If this fails the default handler applies and Ctrl-C ends the process
        // outright. Every link is flushed and fsynced before its request is
        // answered, so an abrupt exit still loses nothing.
        unsafe {
            SetConsoleCtrlHandler(Some(on_console_event), TRUE);
        }
    }

    /// No `poll` here on purpose -- see the module comment. The listener is
    /// non-blocking, so the caller's `accept` returns `WouldBlock` and we simply
    /// tick.
    pub fn wait_for_connection(_listener: &TcpListener, timeout_ms: i32) -> bool {
        thread::sleep(Duration::from_millis(timeout_ms.clamp(1, 100) as u64));
        true
    }

    pub fn detach(cmd: &mut Command) {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    /// A windowless child has no console to send Ctrl-C to, so stopping is
    /// taskkill on the whole tree. x-download records every link as it
    /// finishes, so at most the ones in flight start over next time.
    pub fn interrupt_tree(pid: u32) {
        kill_tree(pid);
    }

    /// Explorer gives a double-clicked console program a console of its own.
    /// Started from a terminal, it shares the shell's, so the count is higher.
    pub fn double_clicked() -> bool {
        let mut ids = [0u32; 4];
        unsafe { GetConsoleProcessList(ids.as_mut_ptr(), ids.len() as u32) == 1 }
    }

    pub fn kill_tree(pid: u32) {
        let _ = Command::new("taskkill")
            .args(["/T", "/F", "/PID", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .status();
    }
}

pub use imp::{
    autostart, detach, detach_console, double_clicked, install_shutdown_handlers, interrupt_tree,
    kill_tree, set_autostart, wait_for_connection,
};

/// Show a folder, or a web address, the way double-clicking it would.
pub fn open(target: &OsStr) -> io::Result<()> {
    let program = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let mut child = Command::new(program)
        .arg(target)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    // Reaped whenever it exits, so a long-running daemon collects no zombies.
    thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}
