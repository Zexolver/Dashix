//! Bridges Android 12+ (API 31+) Material You "dynamic color" -- the
//! wallpaper-derived accent palette -- into Slint. Below API 31, or on
//! any JNI hiccup, this is a no-op: the static default in app.slint is
//! what's shown, since dynamic color is a nice-to-have the rest of the
//! app doesn't depend on.

use jni::objects::{JObject, JValue};
use jni::JavaVM;
use slint::Color;

use crate::MainWindow;

pub fn apply_dynamic_colors(ui: &MainWindow) {
    match try_apply(ui) {
        Ok(true) => log::info!("applied Android dynamic accent color"),
        Ok(false) => log::info!("dynamic color unavailable (pre-Android-12, or no such resource)"),
        Err(e) => log::warn!("dynamic color bridge failed: {e:?}"),
    }
}

fn try_apply(ui: &MainWindow) -> jni::errors::Result<bool> {
    let ctx = ndk_context::android_context();
    let vm = unsafe { JavaVM::from_raw(ctx.vm().cast()) }?;
    let mut env = vm.attach_current_thread()?;
    let activity = unsafe { JObject::from_raw(ctx.context().cast()) };

    let sdk_int = env
        .get_static_field("android/os/Build$VERSION", "SDK_INT", "I")?
        .i()?;
    if sdk_int < 31 {
        return Ok(false);
    }

    // Material You exposes its dynamic palette as framework color
    // resources named system_accent1_100..900 (and accent2/3, neutral1/2)
    // -- see https://developer.android.com/develop/ui/views/theming/dynamic-colors.
    // system_accent1_500 is a reasonable single "the accent color" pick.
    let Some(argb) = resolve_color(&mut env, &activity, "system_accent1_500")? else {
        return Ok(false);
    };

    let color = Color::from_argb_u8(
        (argb >> 24) as u8,
        (argb >> 16) as u8,
        (argb >> 8) as u8,
        argb as u8,
    );
    ui.set_dynamic_accent(color);
    Ok(true)
}

/// Looks up an Android framework color resource by name (e.g.
/// "system_accent1_500") without needing its numeric resource ID -- those
/// IDs aren't stable across Android versions -- then resolves it to an
/// ARGB int via the activity's Resources + Theme.
fn resolve_color(
    env: &mut jni::JNIEnv,
    activity: &JObject,
    name: &str,
) -> jni::errors::Result<Option<i32>> {
    let resources = env
        .call_method(
            activity,
            "getResources",
            "()Landroid/content/res/Resources;",
            &[],
        )?
        .l()?;

    let name_jstr = env.new_string(name)?;
    let type_jstr = env.new_string("color")?;
    let package_jstr = env.new_string("android")?;

    let res_id = env
        .call_method(
            &resources,
            "getIdentifier",
            "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)I",
            &[
                JValue::Object(&JObject::from(name_jstr)),
                JValue::Object(&JObject::from(type_jstr)),
                JValue::Object(&JObject::from(package_jstr)),
            ],
        )?
        .i()?;
    if res_id == 0 {
        return Ok(None);
    }

    let theme = env
        .call_method(
            activity,
            "getTheme",
            "()Landroid/content/res/Resources$Theme;",
            &[],
        )?
        .l()?;

    let color = env
        .call_method(
            &resources,
            "getColor",
            "(ILandroid/content/res/Resources$Theme;)I",
            &[JValue::Int(res_id), JValue::Object(&theme)],
        )?
        .i()?;

    Ok(Some(color))
}
