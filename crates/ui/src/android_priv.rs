//! Launches `pocketserver-daemon` with elevated privileges on Android,
//! where there's no init/systemd to run it as a background service and
//! the app itself has no permission to bind low ports or read other
//! processes' network state without help.
//!
//! Two paths were considered:
//!
//! 1. **Shizuku** (no root required -- the user grants access once via
//!    the separate Shizuku app). NOT implemented here: Shizuku's IPC is
//!    an AIDL/Binder API (`rikka.shizuku:api`), which is Java/Kotlin-only.
//!    Driving it from Rust needs a small JNI shim class bundled into the
//!    APK, which in turn needs a real Gradle-based Android project rather
//!    than `cargo-apk`'s pure-Rust packaging -- a bigger structural change
//!    than this milestone's scope. Left as documented follow-up work.
//! 2. **`su`** (rooted devices/emulators, no extra app required) --
//!    implemented below, and it's the one that actually runs today.

use std::io;
use std::process::{Child, Command};
use std::thread;
use std::time::Duration;

/// Where the daemon binary is expected once pushed to the device (it
/// ships as a separate `aarch64-linux-android` binary alongside the APK,
/// e.g. via `adb push target/aarch64-linux-android/release/pocketserver-daemon
/// /data/local/tmp/pocketserver-daemon` -- it isn't bundled inside the
/// APK itself, since it's a standalone process, not a JNI library).
pub const DAEMON_PATH: &str = "/data/local/tmp/pocketserver-daemon";

/// Best-effort: starts the daemon via `su -c`, logging (not panicking) on
/// failure, since plenty of real devices are neither rooted nor running
/// Shizuku, and the UI should still be usable purely to look at cached
/// state / configure things for next time in that case.
pub fn ensure_daemon_running() {
    match spawn_daemon_with_root() {
        Ok(child) => log::info!("pocketserver-daemon started via su (pid {:?})", child.id()),
        Err(e) => log::warn!(
            "could not start pocketserver-daemon via su ({e}); is the device rooted, \
             and is the daemon binary present at {DAEMON_PATH}?"
        ),
    }
}

/// `su` implementations disagree on invocation syntax: Magisk/SuperSU (most
/// real rooted phones) accept shell-style `su -c "command"`; AOSP's own
/// eng/userdebug toybox `su` (what Android emulator system images ship,
/// confirmed on a real emulator while building this) rejects `-c` --
/// `su: invalid uid/gid '-c'` -- and wants `su <uid> command...` instead.
/// Try the common case first; a rejected `su` invocation exits almost
/// immediately with a usage error, while a real spawned daemon keeps
/// running, so a short liveness check picks between the two.
fn spawn_daemon_with_root() -> io::Result<Child> {
    let mut child = Command::new("su").arg("-c").arg(DAEMON_PATH).spawn()?;

    thread::sleep(Duration::from_millis(300));
    match child.try_wait()? {
        None => Ok(child),
        Some(_) => Command::new("su").arg("0").arg(DAEMON_PATH).spawn(),
    }
}
