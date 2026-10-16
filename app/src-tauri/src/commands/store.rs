//! A copy installed from the Microsoft Store is updated by the Store, never by the updater plugin:
//! it asks the Store what it holds and sends the user there (ADR-0097).

use std::path::Path;

use serde::Serialize;

use super::off_thread;
use crate::error::UiResult;

/// The Store's answer for a copy installed from it.
#[derive(Debug, Serialize)]
pub struct StoreCheck {
    /// The newer package's version as `major.minor.patch`; `None` when the Store holds nothing newer.
    pub version: Option<String>,
}

/// `None` for any copy not installed from the Store, which the updater plugin checks instead.
#[tauri::command]
pub async fn update_store_check() -> UiResult<Option<StoreCheck>> {
    if !from_store() {
        return Ok(None);
    }
    off_thread(|| package::check().map(|version| Some(StoreCheck { version }))).await
}

/// Opens this app's page in the Store app, whose own Update button installs the package.
#[tauri::command]
pub async fn update_store_open() -> UiResult<()> {
    off_thread(package::open).await
}

fn from_store() -> bool {
    cfg!(windows) && std::env::current_exe().is_ok_and(|exe| in_store_package(&exe))
}

/// Windows installs every MSIX package under a `WindowsApps` directory, on whichever drive.
fn in_store_package(exe: &Path) -> bool {
    exe.components()
        .any(|part| part.as_os_str().eq_ignore_ascii_case("WindowsApps"))
}

#[cfg(windows)]
mod package {
    use windows::ApplicationModel::Package;
    use windows::Foundation::Uri;
    use windows::Services::Store::StoreContext;
    use windows::System::Launcher;
    use windows::core::HSTRING;

    use crate::error::{UiError, UiResult};

    fn failed(error: windows::core::Error) -> UiError {
        UiError::Network {
            message: format!("the Microsoft Store did not answer: {error}"),
        }
    }

    pub fn check() -> UiResult<Option<String>> {
        let updates = StoreContext::GetDefault()
            .and_then(|store| store.GetAppAndOptionalStorePackageUpdatesAsync())
            .and_then(|look| look.get())
            .map_err(failed)?;
        let Some(update) = updates.into_iter().next() else {
            return Ok(None);
        };
        let version = update
            .Package()
            .and_then(|package| package.Id())
            .and_then(|id| id.Version())
            .map_err(failed)?;
        // The package carries the release as `major.minor.patch.0`; see release.yml.
        Ok(Some(format!(
            "{}.{}.{}",
            version.Major, version.Minor, version.Build
        )))
    }

    /// By package family name, so the product id the Store assigned lives in one place only.
    pub fn open() -> UiResult<()> {
        let family = Package::Current()
            .and_then(|package| package.Id())
            .and_then(|id| id.FamilyName())
            .map_err(failed)?;
        Uri::CreateUri(&HSTRING::from(format!("ms-windows-store://pdp/?PFN={family}")))
            .and_then(|uri| Launcher::LaunchUriAsync(&uri))
            .and_then(|launch| launch.get())
            .map(|_| ())
            .map_err(failed)
    }
}

#[cfg(not(windows))]
mod package {
    use crate::error::{UiError, UiResult};

    pub fn check() -> UiResult<Option<String>> {
        Err(UiError::invalid(
            "only a Windows copy is installed from the Microsoft Store",
        ))
    }

    pub fn open() -> UiResult<()> {
        check().map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_store_package_is_found_on_any_drive_and_a_home_install_is_not() {
        assert!(in_store_package(Path::new(
            "C:/Program Files/WindowsApps/VladimirVarvorkin.Stonqs_0.6.0.0_x64__abc/Stonqs.exe"
        )));
        assert!(in_store_package(Path::new("D:/windowsapps/Stonqs/Stonqs.exe")));
        assert!(!in_store_package(Path::new(
            "C:/Users/me/AppData/Local/Stonqs/Stonqs.exe"
        )));
    }
}
