/** Settings, profiles, UI state, notices and development helpers. */

import { openUrl } from "@tauri-apps/plugin-opener";
import type { DevAlertStep, AppSettings, Profile, ProfileList } from "../types";
import { call } from "./core";

export const appApi = {
  /** Writes the licence notices of every dependency where the user picked. */
  noticesSave: (path: string) => call<void>("notices_save", { path }),

  /** Hands an address to the OS: a webview opens no window of its own on any platform. */
  openUrl: (url: string) => openUrl(url),

  settingsGet: () => call<AppSettings>("settings_get"),

  profilesList: () => call<ProfileList>("profiles_list"),
  profileCreate: (name: string) => call<Profile>("profile_create", { name }),
  profileRename: (id: string, name: string) => call<Profile>("profile_rename", { id, name }),
  /** Deletes the *open* profile (its password, when it has one) and moves to another. */
  profileDelete: (password: string | null) => call<void>("profile_delete", { password }),
  /** Swaps the host's open profile. The caller reloads the window: everything cached is the old one's. */
  profileOpen: (id: string) => call<void>("profile_open", { id }),
  profileUnlock: (password: string, remember: boolean) =>
    call<void>("profile_unlock", { password, remember }),
  profileLock: () => call<void>("profile_lock"),
  /** The first password when `current` is null, a change otherwise. */
  profileSetPassword: (current: string | null, password: string) =>
    call<void>("profile_set_password", { current, password }),
  profileRemovePassword: (current: string) => call<void>("profile_remove_password", { current }),
  profileRemember: (remember: boolean) => call<void>("profile_remember", { remember }),
  settingsSave: (settings: AppSettings) => call<AppSettings>("settings_save", { settings }),
  /** Saves dashboard layout separately from general settings. */
  uiStateSave: (ui: unknown) => call<void>("ui_state_save", { ui }),

  /** Fills an empty portfolio with the sample history; refuses once it holds an account. */
  demoSeed: () => call<void>("demo_seed"),
  /** Debug builds only: moves a real quote across a rule's level; returns crossings logged. */
  devAlertSimulate: (alert_id: string, step: DevAlertStep) =>
    call<number>("dev_alert_simulate", { alertId: alert_id, step }),
};
