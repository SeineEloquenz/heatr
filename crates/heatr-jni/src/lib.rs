use std::os::fd::{FromRawFd, OwnedFd};

use futures_util::{StreamExt, pin_mut};
use heatr::{
    HeatItDevice, UsbBulkTransferDevice,
    heat_it::HeatingPhase,
    prefs::{Duration, Generation, Preferences, SkinSensitivity},
    support::SUPPORT_STATEMENTS,
};
use jni::{
    EnvUnowned,
    errors::ThrowRuntimeExAndDefault,
    objects::{JClass, JObject, JValue},
    sys::{jint, jintArray, jlong},
    {jni_sig, jni_str},
};
#[cfg(target_os = "android")]
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug)]
struct NativeError(String);

impl std::fmt::Display for NativeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for NativeError {}

impl From<jni::errors::Error> for NativeError {
    fn from(e: jni::errors::Error) -> Self {
        Self(e.to_string())
    }
}

impl From<heatr::HeatrError> for NativeError {
    fn from(e: heatr::HeatrError) -> Self {
        Self(e.to_string())
    }
}

#[cfg(target_os = "android")]
fn init_logging() {
    tracing_subscriber::registry()
        .with(tracing_android::layer("heatr").unwrap())
        .init();
}

#[cfg(not(target_os = "android"))]
fn init_logging() {
    tracing_subscriber::fmt().init();
}

/// Called by the JVM when the native library is first loaded.
#[unsafe(no_mangle)]
pub extern "system" fn JNI_OnLoad(
    _vm: *mut jni::sys::JavaVM,
    _reserved: *mut std::ffi::c_void,
) -> jni::sys::jint {
    init_logging();
    // Android only supports JNI 1.6
    jni::sys::JNI_VERSION_1_6
}

/// Opens a `HeatItDevice` from a file descriptor obtained via
/// `UsbDeviceConnection.getFileDescriptor()`.
///
/// Returns an opaque `Long` handle. The caller must pass this to subsequent
/// JNI calls and eventually call `closeDevice` to free it.
/// Throws `RuntimeException` on error.
#[unsafe(no_mangle)]
pub extern "system" fn Java_nz_eloque_heatr_native_HeatrJni_openDevice<'local>(
    mut unowned_env: EnvUnowned<'local>,
    _: JClass<'local>,
    fd: jint,
) -> jlong {
    unowned_env
        .with_env(|_env| -> Result<jlong, NativeError> {
            // Duplicate the fd so Rust owns an independent copy. The original fd
            // stays open in Kotlin's UsbDeviceConnection for the duration of the
            // session; Rust's copy is closed when the HeatItDevice is dropped.
            let dup = unsafe { libc::dup(fd) };
            if dup < 0 {
                return Err(NativeError("dup(fd) failed".to_owned()));
            }
            let owned = unsafe { OwnedFd::from_raw_fd(dup) };
            // The heatr library is async; JNI entry points are called from
            // Kotlin background dispatchers, so blocking here is fine.
            let backend = pollster::block_on(UsbBulkTransferDevice::from_fd(owned))?;
            Ok(Box::into_raw(Box::new(HeatItDevice::new(Box::new(backend)))) as jlong)
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

/// Releases a device handle returned by `openDevice`. Safe to call with 0.
#[unsafe(no_mangle)]
pub extern "system" fn Java_nz_eloque_heatr_native_HeatrJni_closeDevice<'local>(
    _: EnvUnowned<'local>,
    _: JClass<'local>,
    handle: jlong,
) {
    if handle != 0 {
        unsafe { drop(Box::from_raw(handle as *mut HeatItDevice)) };
    }
}

/// Runs the full device initialization sequence (`self_test`).
/// Throws `RuntimeException` on error.
#[unsafe(no_mangle)]
pub extern "system" fn Java_nz_eloque_heatr_native_HeatrJni_runInit<'local>(
    mut unowned_env: EnvUnowned<'local>,
    _: JClass<'local>,
    handle: jlong,
) {
    unowned_env
        .with_env(|_env| -> Result<(), NativeError> {
            if handle == 0 {
                return Err(NativeError("null device handle".to_owned()));
            }
            let device = unsafe { &mut *(handle as *mut HeatItDevice) };
            pollster::block_on(device.self_test())?;
            Ok(())
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

/// Starts a heating cycle and blocks until it completes, calling
/// `callback.onProgress(phase: Int, temperature: Int)` after each poll.
///
/// Phase constants: 0 = Heating, 1 = Applying, 2 = Done.
/// Throws `RuntimeException` on error.
#[unsafe(no_mangle)]
pub extern "system" fn Java_nz_eloque_heatr_native_HeatrJni_startHeating<'local>(
    mut unowned_env: EnvUnowned<'local>,
    _: JClass<'local>,
    handle: jlong,
    duration: jint,
    generation: jint,
    skin_sensitivity: jint,
    callback: JObject<'local>,
) {
    unowned_env
        .with_env(|env| -> Result<(), NativeError> {
            if handle == 0 {
                return Err(NativeError("null device handle".to_owned()));
            }

            let &duration = [Duration::Short, Duration::Medium, Duration::Long]
                .get(duration as usize)
                .ok_or_else(|| NativeError("invalid duration value".to_owned()))?;
            let &generation = [Generation::Child, Generation::Adult]
                .get(generation as usize)
                .ok_or_else(|| NativeError("invalid generation value".to_owned()))?;
            let &skin_sensitivity = [SkinSensitivity::Sensitive, SkinSensitivity::Regular]
                .get(skin_sensitivity as usize)
                .ok_or_else(|| NativeError("invalid skin_sensitivity value".to_owned()))?;

            let prefs = Preferences {
                duration,
                generation,
                skin_sensitivity,
            };
            let device = unsafe { &mut *(handle as *mut HeatItDevice) };

            let jvm = env.get_java_vm()?;
            let callback_ref = env.new_global_ref(&callback)?;

            // The heatr library is async; this entry point is called from a
            // Kotlin background dispatcher, so blocking on the whole heating
            // cycle is fine (and matches the documented contract).
            pollster::block_on(async {
                device.start_with_preferences(&prefs).await?;

                {
                    let stream = device.monitor();
                    pin_mut!(stream);
                    while let Some(status) = stream.next().await {
                        let status = status?;
                        let phase: i32 = match status.phase {
                            HeatingPhase::Heating => 0,
                            HeatingPhase::Applying => 1,
                            HeatingPhase::Done => 2,
                        };
                        if let Err(e) =
                            jvm.attach_current_thread(|env| -> Result<(), jni::errors::Error> {
                                env.call_method(
                                    callback_ref.as_obj(),
                                    jni_str!("onProgress"),
                                    jni_sig!("(II)V"),
                                    &[
                                        JValue::Int(phase),
                                        JValue::Int(status.temperature.as_celsius() as i32),
                                    ],
                                )?;
                                Ok(())
                            })
                        {
                            log::error!("JNI progress callback failed: {e}");
                        }
                    }
                }

                // The monitor stream no longer auto-stops the device once the
                // cycle ends; send STOP_HEATING explicitly.
                device.stop_heating().await?;
                Ok::<(), NativeError>(())
            })?;

            Ok(())
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

/// Returns the supported VID/PID pairs as a flat `IntArray`:
/// `[vid0, pid0, vid1, pid1, …]`.
#[unsafe(no_mangle)]
pub extern "system" fn Java_nz_eloque_heatr_native_HeatrJni_getSupportedVidPids<'local>(
    mut unowned_env: EnvUnowned<'local>,
    _: JClass<'local>,
) -> jintArray {
    unowned_env
        .with_env(|env| -> Result<jintArray, NativeError> {
            let data: Vec<i32> = SUPPORT_STATEMENTS
                .iter()
                .filter(|s| s.supported)
                .flat_map(|s| [s.vid as i32, s.pid as i32])
                .collect();
            let arr = env.new_int_array(data.len())?;
            arr.set_region(env, 0, &data)?;
            Ok(arr.as_raw())
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}
