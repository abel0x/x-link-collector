// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 abel0x <https://github.com/abel0x>

//! The two things this daemon needs from the operating system: a way to wait
//! for a connection without spinning, and a way to hear about shutdown.
//!
//! Unix gets `poll(2)` and `signal(2)`, declared here rather than pulled in as
//! a dependency. Windows gets a short sleep between non-blocking accepts and a
//! console control handler. That asymmetry is deliberate: the Windows build is
//! cross-checked but cannot be run from the machine this was written on, so it
//! uses the version that has no way to be subtly wrong. At a few requests a
//! minute, a 25 ms tick costs nothing measurable either way.

use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};

/// Set once a shutdown signal arrives; the accept loop checks it between waits.
pub static SHUTDOWN: AtomicBool = AtomicBool::new(false);

pub fn shutting_down() -> bool {
    SHUTDOWN.load(Ordering::Relaxed)
}

#[cfg(unix)]
mod imp {
    use super::*;
    use std::os::unix::io::AsRawFd;

    const SIGHUP: i32 = 1;
    const SIGINT: i32 = 2;
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
    use std::thread;
    use std::time::Duration;

    /// Ctrl-C, Ctrl-Break, console closed, logoff, shutdown.
    const HANDLED_EVENTS: [u32; 5] = [0, 1, 2, 5, 6];
    const TRUE: i32 = 1;

    type HandlerRoutine = unsafe extern "system" fn(u32) -> i32;

    extern "system" {
        fn SetConsoleCtrlHandler(handler: Option<HandlerRoutine>, add: i32) -> i32;
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
}

pub use imp::{install_shutdown_handlers, wait_for_connection};
