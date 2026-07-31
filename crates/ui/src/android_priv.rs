//! Launches `pocketserver-daemon` on Android without requiring root or
//! Shizuku, by bundling it inside the APK itself rather than treating it
//! as something the user has to separately push to the device.
//!
//! The trick: cargo-apk's `runtime_libs` (see Cargo.toml) copies
//! `runtime-libs/<abi>/libpocketserver_daemon.so` into the APK's
//! `lib/<abi>/` directory. Android doesn't actually check that files
//! there are real shared libraries -- it just extracts them (with
//! `extractNativeLibs = true`, also set in Cargo.toml) to a real,
//! executable-permitted path on disk as part of installing the signed,
//! verified APK. So a plain ELF *executable* renamed to look like a
//! `.so` gets the same treatment, and can be run directly via
//! `ApplicationInfo.nativeLibraryDir` -- no `su`, no Shizuku, no runtime
//! download. A genuine runtime download-then-exec approach wouldn't work
//! here anyway: Android's W^X enforcement (since Android 10) blocks
//! executing a file an app wrote to its own storage at runtime, which is
//! exactly why bundling it as part of the APK instead is the fix, not a
//! workaround.
//!
//! This only gets the control-plane daemon (port 7878, unprivileged)
//! running. Getting rpxy/rpxy-l4 to bind privileged ports (80/443) on
//! Android -- which those two processes, not this one, would need -- is
//! a separate, still-unaddressed concern; root/Shizuku would matter
//! there, not here.

use std::path::PathBuf;
use std::process::{Child, Command};

use jni::objects::{JObject, JString};
use jni::JavaVM;

const BUNDLED_DAEMON_SO_NAME: &str = "libpocketserver_daemon.so";

fn native_daemon_path() -> jni::errors::Result<PathBuf> {
    let ctx = ndk_context::android_context();
    let vm = unsafe { JavaVM::from_raw(ctx.vm().cast()) }?;
    let mut env = vm.attach_current_thread()?;
    let activity = unsafe { JObject::from_raw(ctx.context().cast()) };

    let app_info = env
        .call_method(
            &activity,
            "getApplicationInfo",
            "()Landroid/content/pm/ApplicationInfo;",
            &[],
        )?
        .l()?;
    let dir_obj = env
        .get_field(&app_info, "nativeLibraryDir", "Ljava/lang/String;")?
        .l()?;
    let dir_jstring = JString::from(dir_obj);
    let dir: String = env.get_string(&dir_jstring)?.into();

    Ok(PathBuf::from(dir).join(BUNDLED_DAEMON_SO_NAME))
}

fn spawn_bundled_daemon() -> anyhow::Result<Child> {
    let path = native_daemon_path().map_err(|e| anyhow::anyhow!("resolving daemon path: {e:?}"))?;
    Command::new(&path)
        .spawn()
        .map_err(|e| anyhow::anyhow!("spawning {}: {e}", path.display()))
}

/// Best-effort daemon start, returning a human-readable status the UI can
/// show directly -- used both for the automatic attempt at app startup
/// and the explicit "Start daemon" button for the user to retry/restart it.
pub fn ensure_daemon_running() -> String {
    match spawn_bundled_daemon() {
        Ok(child) => {
            let msg = format!("daemon started (pid {:?})", child.id());
            log::info!("{msg}");
            msg
        }
        Err(e) => {
            let msg = format!("daemon start failed: {e}");
            log::warn!("{msg}");
            msg
        }
    }
}
