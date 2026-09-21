//! Profiles: list, create, rename, delete, open. Opening one swaps what `AppState` holds; the
//! frontend then reloads itself, because every cached answer it has belongs to the old profile.

use crate::error::UiResult;
use crate::profiles::Profile;
use crate::state::AppState;
use serde::Serialize;
use tauri::{AppHandle, State};

#[derive(Debug, Serialize)]
pub struct ProfileView {
    #[serde(flatten)]
    pub profile: Profile,
    /// Whether it has a password.
    pub protected: bool,
}

#[derive(Debug, Serialize)]
pub struct ProfileList {
    pub profiles: Vec<ProfileView>,
    /// Id of the open profile.
    pub open: String,
    /// The open profile has a password and has not been unlocked in this run.
    pub locked: bool,
    /// This device opens the open profile without asking for its password.
    pub remembered: bool,
}

#[tauri::command]
pub fn profiles_list(state: State<AppState>) -> UiResult<ProfileList> {
    let profiles = state
        .profiles
        .list()?
        .into_iter()
        .map(|profile| ProfileView {
            protected: crate::vault::is_protected(crate::profiles::folder_of(
                &state.profiles.db_path(&profile.id),
            )),
            profile,
        })
        .collect();
    Ok(ProfileList {
        profiles,
        open: state.profile_id()?,
        locked: state.is_locked(),
        remembered: state.is_remembered(),
    })
}

#[tauri::command]
pub fn profile_create(state: State<AppState>, name: String) -> UiResult<Profile> {
    state.profiles.create(&name)
}

#[tauri::command]
pub fn profile_rename(state: State<AppState>, id: String, name: String) -> UiResult<Profile> {
    state.profiles.rename(&id, &name)
}

/// Deletes the open profile and moves to another. There is no way to delete a profile from
/// outside it — a protected one also asks for its password again. Async: that check is Argon2.
#[tauri::command]
pub async fn profile_delete(state: State<'_, AppState>, password: Option<String>) -> UiResult<()> {
    state.delete_open_profile(password.as_deref())
}

/// Opens another profile in place of this one, and gives it the refresh a start would have.
#[tauri::command]
pub fn profile_open(app: AppHandle, state: State<AppState>, id: String) -> UiResult<()> {
    if id == state.profile_id()? {
        return Ok(());
    }
    state.open_profile(&id)?;
    if state.settings()?.due(chrono::Utc::now()) {
        crate::jobs::start(&app, &state, crate::jobs::RefreshMode::CatchUp);
    }
    Ok(())
}

/// Async so the Argon2 derivation (a deliberate fraction of a second) runs off the main thread.
#[tauri::command]
pub async fn profile_unlock(
    app: AppHandle,
    state: State<'_, AppState>,
    password: String,
    remember: bool,
) -> UiResult<()> {
    state.unlock(&password, remember)?;
    if state.settings()?.due(chrono::Utc::now()) {
        crate::jobs::start(&app, &state, crate::jobs::RefreshMode::CatchUp);
    }
    Ok(())
}

#[tauri::command]
pub fn profile_lock(state: State<AppState>) -> UiResult<()> {
    state.lock()
}

/// Sets the first password, or changes it — `current` is required for a change.
#[tauri::command]
pub async fn profile_set_password(
    state: State<'_, AppState>,
    current: Option<String>,
    password: String,
) -> UiResult<()> {
    state.set_password(current.as_deref(), &password)
}

#[tauri::command]
pub async fn profile_remove_password(state: State<'_, AppState>, current: String) -> UiResult<()> {
    state.remove_password(&current)
}

#[tauri::command]
pub fn profile_remember(state: State<AppState>, remember: bool) -> UiResult<()> {
    state.set_remembered(remember)
}
