//! OS-level notifications (Windows toast / macOS Notification Center / Linux
//! D-Bus). Shown over the whole screen, regardless of which app is focused,
//! when a clip is saved.

use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};
#[cfg(not(target_os = "windows"))]
use tauri_plugin_notification::NotificationExt;

/// AUMID used for Windows toast notifications. Must match the bundle
/// `identifier` in `tauri.conf.json` — the notification plugin uses it as the
/// toast's `System.AppUserModel.ID`.
#[cfg(target_os = "windows")]
const AUMID: &str = "com.dac63.prism";

/// Embedded Prism icon, extracted at startup and referenced as the toast logo.
#[cfg(target_os = "windows")]
const TOAST_ICON_BYTES: &[u8] = include_bytes!("../icons/128x128.png");

/// Show a native "Clip saved" notification for `output_path`.
pub fn notify_clip_saved(app: &AppHandle, output_path: &Path) {
    let filename = output_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| output_path.to_string_lossy().to_string());

    #[cfg(target_os = "windows")]
    {
        // Render the toast as "Prism" with the logo regardless of launch path.
        // tauri-plugin-notification deliberately skips setting the toast's
        // AppUserModelID when the exe runs from target/debug or target/release
        // (see its desktop.rs), and notify-rust then falls back to
        // PowerShell's AUMID — that's why the raw exe showed "PowerShell".
        // Building the notification through notify-rust directly with our
        // registered AUMID fixes the attribution.
        register_aumid(app);

        let mut notification = notify_rust::Notification::new();
        notification
            .app_id(AUMID)
            .summary("Clip saved")
            .body(&filename);
        tauri::async_runtime::spawn(async move {
            if let Err(e) = notification.show() {
                eprintln!("[notification] failed to show clip notification: {e}");
            }
        });
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Err(e) = app
            .notification()
            .builder()
            .title("Clip saved")
            .body(filename)
            .show()
        {
            eprintln!("[notification] failed to show clip notification: {e}");
        }
    }
}

/// Register the AppUserModelID so Windows toasts render even when the app is
/// launched outside the installer's Start-Menu shortcut. Windows only renders
/// a toast if its AUMID is registered on the system; the NSIS/MSI shortcut
/// tag is fragile (lost on repair, moved exe, running the raw binary). This
/// self-heals at every launch:
///   1. `HKCU\Software\Classes\AppUserModelId\<id>` with a DisplayName
///      (per-user, no admin).
///   2. `IconUri` pointing at the embedded Prism icon (extracted to the app
///      data dir) so the toast shows the logo, not a generic placeholder.
///   3. `SetCurrentProcessExplicitAppUserModelID` pins the process to it.
#[cfg(target_os = "windows")]
pub fn register_aumid(app: &AppHandle) {
    use windows::core::PCWSTR;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE,
        REG_OPTION_NON_VOLATILE, REG_SZ,
    };
    use windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;

    let icon_uri = ensure_toast_icon(app);

    // Create a Start Menu shortcut carrying the AUMID so Windows can resolve
    // the app's display name ("Prism") and icon for toasts. Without it, an
    // unpackaged app's toast renders with the raw AUMID as the name and no
    // logo — the registry DisplayName/IconUri alone aren't honored for the
    // toast header.
    ensure_start_menu_shortcut();

    let app_id = encode_wide(AUMID);
    let key_path = encode_wide(r"Software\Classes\AppUserModelId\com.dac63.prism");
    let display_name = encode_wide("Prism");
    let value_name = encode_wide("DisplayName");
    let icon_uri_name = encode_wide("IconUri");
    let icon_bg_name = encode_wide("IconBackgroundColor");
    let icon_bg = encode_wide("0");

    unsafe {
        // 1. Register the AUMID under HKCU so the Action Center renders toasts.
        let mut key: HKEY = HKEY(std::ptr::null_mut());
        let status = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(key_path.as_ptr()),
            Some(0),
            PCWSTR(std::ptr::null()),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            Some(std::ptr::null()),
            &mut key,
            None,
        );
        if status.is_ok() {
            let data: &[u8] = std::slice::from_raw_parts(
                display_name.as_ptr().cast(),
                display_name.len() * std::mem::size_of::<u16>(),
            );
            let _ = RegSetValueExW(
                key,
                PCWSTR(value_name.as_ptr()),
                Some(0),
                REG_SZ,
                Some(data),
            );

            // 2. Toast logo + background color.
            if let Some(icon_path) = &icon_uri {
                let icon_uri_wide = encode_wide(&icon_path.to_string_lossy());
                let data: &[u8] = std::slice::from_raw_parts(
                    icon_uri_wide.as_ptr().cast(),
                    icon_uri_wide.len() * std::mem::size_of::<u16>(),
                );
                let _ = RegSetValueExW(
                    key,
                    PCWSTR(icon_uri_name.as_ptr()),
                    Some(0),
                    REG_SZ,
                    Some(data),
                );
            }
            let data: &[u8] = std::slice::from_raw_parts(
                icon_bg.as_ptr().cast(),
                icon_bg.len() * std::mem::size_of::<u16>(),
            );
            let _ = RegSetValueExW(
                key,
                PCWSTR(icon_bg_name.as_ptr()),
                Some(0),
                REG_SZ,
                Some(data),
            );
            let _ = RegCloseKey(key);
        } else {
            eprintln!("[notification] failed to create AUMID registry key: {status:?}");
        }

        // 3. Pin the current process to the AUMID.
        let result = SetCurrentProcessExplicitAppUserModelID(PCWSTR(app_id.as_ptr()));
        if result.is_err() {
            eprintln!("[notification] failed to set explicit AppUserModelID: {result:?}");
        }
    }
}

/// Create a Start Menu shortcut for Prism tagged with the AUMID
/// (`System.AppUserModel.ID`), so the toast notification platform resolves the
/// app's display name and logo. For unpackaged Win32 apps this shortcut is what
/// Windows reads when rendering a toast for a custom AUMID — the registry
/// `DisplayName`/`IconUri` values alone fall back to showing the raw AUMID.
/// The installer already ships a shortcut (same name/path), so when the app is
/// installed this is a no-op.
#[cfg(target_os = "windows")]
fn ensure_start_menu_shortcut() {
    use windows::core::{Interface, PCWSTR, PWSTR};
    use windows::Win32::Storage::EnhancedStorage::PKEY_AppUserModel_ID;
    use windows::Win32::System::Com::StructuredStorage::{
        PROPVARIANT, PROPVARIANT_0, PROPVARIANT_0_0, PROPVARIANT_0_0_0,
    };
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoTaskMemAlloc, CoTaskMemFree, CoUninitialize,
        IPersistFile, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::System::Variant::VT_LPWSTR;
    use windows::Win32::UI::Shell::{IShellLinkW, PropertiesSystem::IPropertyStore, ShellLink};

    let exe = match std::env::current_exe() {
        Ok(path) => path,
        Err(e) => {
            eprintln!("[notification] failed to resolve current exe: {e}");
            return;
        }
    };
    let start_menu = match dirs::data_dir() {
        Some(dir) => dir,
        None => {
            eprintln!("[notification] failed to resolve start menu dir");
            return;
        }
    };
    let programs = start_menu
        .join("Microsoft")
        .join("Windows")
        .join("Start Menu")
        .join("Programs");
    if let Err(e) = std::fs::create_dir_all(&programs) {
        eprintln!("[notification] failed to create start menu dir: {e}");
        return;
    }
    let lnk_path = programs.join("Prism.lnk");
    if lnk_path.exists() {
        return;
    }

    let exe_str = exe.to_string_lossy().to_string();
    let wd = exe
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| exe_str.clone());
    let lnk_str = lnk_path.to_string_lossy().to_string();

    unsafe {
        // Creating a ShellLink requires COM initialized on this thread (the
        // app's setup thread hasn't initialized it, unlike the tokio workers
        // used by notify-rust). S_FALSE means COM was already initialized
        // here, so only uninitialize when we actually initialized it.
        let co = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        if co.is_err() {
            eprintln!("[notification] failed to initialize COM: {co}");
            return;
        }
        let initialized_here = co.0 == 0;

        // Memory for the AppUserModel.ID value. Must outlive the property
        // store (it references the string until released), so it's freed only
        // after all interfaces are dropped below.
        let mut aumid_mem: *mut u16 = std::ptr::null_mut();

        // Interfaces must be dropped before CoUninitialize below, or COM
        // release calls after uninit corrupt the heap.
        {
            let shell_link: IShellLinkW =
                match CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER) {
                    Ok(link) => link,
                    Err(e) => {
                        eprintln!("[notification] failed to create shell link: {e}");
                        if initialized_here {
                            CoUninitialize();
                        }
                        return;
                    }
                };

            let _ = shell_link.SetPath(PCWSTR(encode_wide(&exe_str).as_ptr()));
            let _ = shell_link.SetWorkingDirectory(PCWSTR(encode_wide(&wd).as_ptr()));
            let _ = shell_link.SetDescription(PCWSTR(encode_wide("Prism").as_ptr()));
            // The exe carries the embedded Prism icon (from icon.ico), which the
            // toast platform uses as the app logo.
            let _ = shell_link.SetIconLocation(PCWSTR(encode_wide(&exe_str).as_ptr()), 0);

            if let Ok(store) = shell_link.cast::<IPropertyStore>() {
                let aumid_wide = encode_wide(AUMID);
                aumid_mem = CoTaskMemAlloc(aumid_wide.len() * 2) as *mut u16;
                if !aumid_mem.is_null() {
                    std::ptr::copy_nonoverlapping(aumid_wide.as_ptr(), aumid_mem, aumid_wide.len());
                    let propvar = PROPVARIANT {
                        Anonymous: PROPVARIANT_0 {
                            Anonymous: std::mem::ManuallyDrop::new(PROPVARIANT_0_0 {
                                vt: VT_LPWSTR,
                                wReserved1: 0,
                                wReserved2: 0,
                                wReserved3: 0,
                                Anonymous: PROPVARIANT_0_0_0 {
                                    pwszVal: PWSTR(aumid_mem),
                                },
                            }),
                        },
                    };
                    let _ = store.SetValue(&PKEY_AppUserModel_ID, &propvar);
                    let _ = store.Commit();
                }
            }

            if let Ok(file) = shell_link.cast::<IPersistFile>() {
                let _ = file.Save(PCWSTR(encode_wide(&lnk_str).as_ptr()), true);
            }
        }

        if !aumid_mem.is_null() {
            CoTaskMemFree(Some(aumid_mem as *mut core::ffi::c_void));
        }
        if initialized_here {
            CoUninitialize();
        }
    }
}

/// Extract the embedded Prism icon to the app data dir so Windows can render
/// it as the toast logo via the AUMID's `IconUri` registry value.
#[cfg(target_os = "windows")]
fn ensure_toast_icon(app: &AppHandle) -> Option<PathBuf> {
    let dir = match app.path().app_data_dir() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("[notification] failed to resolve app data dir: {e}");
            return None;
        }
    };
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("[notification] failed to create app data dir: {e}");
        return None;
    }
    let path = dir.join("prism_toast_icon.png");
    if !path.exists() {
        if let Err(e) = std::fs::write(&path, TOAST_ICON_BYTES) {
            eprintln!("[notification] failed to write toast icon: {e}");
            return None;
        }
    }
    Some(path)
}

#[cfg(target_os = "windows")]
fn encode_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
