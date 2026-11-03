//! Qt's helper is shared by unrelated apps. Only its parent chain can name
//! the Telemost that owns a Windows microphone session.

use std::path::{Path, PathBuf};

pub(super) fn is_qt_helper(path: &Path) -> bool {
    named(path, "QtWebEngineProcess.exe")
}

fn named(path: &Path, executable: &str) -> bool {
    path.file_name()
        .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case(executable))
}

pub(super) fn resolve_windows_helper(
    pid: u32,
    mut parent: impl FnMut(u32) -> Option<(u32, PathBuf)>,
) -> Option<(u32, PathBuf)> {
    let mut current = pid;
    for _ in 0..4 {
        let (owner, path) = parent(current)?;
        if owner == 0 || owner == current || owner == pid {
            return None;
        }
        if named(&path, "YandexTelemost.exe") {
            return Some((owner, path));
        }
        if !is_qt_helper(&path) {
            return None;
        }
        current = owner;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telemost_windows_qt_helpers_require_a_known_owner() {
        assert!(is_qt_helper(Path::new("/app/QTWEBENGINEPROCESS.EXE")));
        assert!(!is_qt_helper(Path::new("/app/OtherQtWebEngineProcess.exe")));
        let resolved = resolve_windows_helper(3, |pid| match pid {
            3 => Some((2, "QtWebEngineProcess.exe".into())),
            2 => Some((1, "YandexTelemost.exe".into())),
            _ => None,
        });
        assert_eq!(resolved, Some((1, "YandexTelemost.exe".into())));
        for name in [
            "OtherQtApp.exe",
            "YandexTelemostUpdateService.exe",
            "telemost.exe",
        ] {
            assert!(resolve_windows_helper(3, |_| Some((2, name.into()))).is_none());
        }
        assert!(resolve_windows_helper(3, |_| None).is_none());
        assert!(resolve_windows_helper(3, |_| Some((3, "YandexTelemost.exe".into()))).is_none());
        let mut calls = 0;
        assert!(
            resolve_windows_helper(10, |pid| {
                calls += 1;
                Some((pid - 1, "QtWebEngineProcess.exe".into()))
            })
            .is_none()
        );
        assert_eq!(calls, 4);
    }
}
