//! Microphone users on Windows: the processes with an active audio session
//! on any capture device, from `IAudioSessionManager2`. Only session
//! metadata is read; no audio is captured.

use color_eyre::eyre::{Result, eyre};
use windows::Win32::Media::Audio::{
    AudioSessionStateActive, DEVICE_STATE_ACTIVE, IAudioSessionControl2, IAudioSessionManager2,
    IMMDeviceEnumerator, MMDeviceEnumerator, eCapture,
};
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
};
use windows::core::Interface;

use super::{ActiveMicrophoneApplication, deduplicated};

/// The processes capturing from a microphone now.
pub fn active_microphone_applications() -> Result<Vec<ActiveMicrophoneApplication>> {
    // SAFETY: Joins this thread to the multithreaded apartment; a thread
    // already in it gets `S_FALSE`, which is fine.
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    let own_pid = std::process::id();
    let mut applications = Vec::new();
    let sessions = (|| -> windows::core::Result<Vec<u32>> {
        // SAFETY: Plain COM calls on interfaces this function owns.
        unsafe {
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let devices = enumerator.EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE)?;
            let mut pids = Vec::new();
            for device in 0..devices.GetCount()? {
                let Ok(device) = devices.Item(device) else {
                    continue;
                };
                let Ok(manager) = device.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None) else {
                    continue;
                };
                let Ok(sessions) = manager.GetSessionEnumerator() else {
                    continue;
                };
                for session in 0..sessions.GetCount()? {
                    let Ok(control) = sessions.GetSession(session) else {
                        continue;
                    };
                    if control.GetState().ok() != Some(AudioSessionStateActive) {
                        continue;
                    }
                    if let Ok(control) = control.cast::<IAudioSessionControl2>()
                        && let Ok(pid) = control.GetProcessId()
                    {
                        pids.push(pid);
                    }
                }
            }
            Ok(pids)
        }
    })()
    .map_err(|error| eyre!("could not read microphone sessions: {error}"))?;
    let mut parents = None;
    for pid in sessions {
        if pid == 0 || pid == own_pid {
            continue;
        }
        let Some(mut path) = process_path(pid) else {
            continue;
        };
        let mut owner_pid = pid;
        if super::telemost::is_qt_helper(&path) {
            let parents = parents.get_or_insert_with(process_parents);
            let Some((owner, owner_path)) = super::telemost::resolve_windows_helper(pid, |child| {
                let parent = *parents.get(&child)?;
                Some((parent, process_path(parent)?))
            }) else {
                continue;
            };
            owner_pid = owner;
            path = owner_path;
        }
        let Some(executable) = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
        else {
            continue;
        };
        let name = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_else(|| executable.clone());
        applications.push(ActiveMicrophoneApplication {
            pid: owner_pid as i32,
            bundle_id: executable,
            name,
        });
    }
    Ok(deduplicated(applications))
}

fn process_path(pid: u32) -> Option<std::path::PathBuf> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
    };
    // SAFETY: The handle is checked, used once, and closed.
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process.is_null() {
            return None;
        }
        let mut buffer = [0_u16; 1024];
        let mut length = buffer.len() as u32;
        let ok = QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut length);
        CloseHandle(process);
        (ok != 0).then(|| String::from_utf16_lossy(&buffer[..length as usize]).into())
    }
}

/// One snapshot per poll, and only when a Qt helper owns a microphone session.
fn process_parents() -> std::collections::HashMap<u32, u32> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    };
    let mut parents = std::collections::HashMap::new();
    // SAFETY: A process-only snapshot with a correctly sized entry; the valid
    // handle is closed on completion. No process is opened for mutation.
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return parents;
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut found = Process32FirstW(snapshot, &mut entry);
        while found != 0 {
            parents.insert(entry.th32ProcessID, entry.th32ParentProcessID);
            found = Process32NextW(snapshot, &mut entry);
        }
        CloseHandle(snapshot);
    }
    parents
}
