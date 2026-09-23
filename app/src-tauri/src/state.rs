//! Shared application state and the current portfolio.

use crate::error::{UiError, UiResult};
use sq_core::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use tauri::{App, Manager};
use zeroize::Zeroizing;

/// Portfolio and accounts selected for an analytics request.
pub struct ScopeSelection {
    pub portfolio: Portfolio,
    /// Accounts included in the selected scope.
    pub accounts: Vec<String>,
}

impl ScopeSelection {
    pub fn analytics<'a>(&'a self, store: &'a Store) -> UiResult<PortfolioAnalytics<'a>> {
        Ok(PortfolioAnalytics::new(store, &self.portfolio)?.scoped_to(&self.accounts))
    }

    /// Every account of the portfolio, whatever the scope is. For facts about an *instrument*
    /// rather than about the lens — a payment schedule does not change with the account picker.
    pub fn whole_portfolio<'a>(&'a self, store: &'a Store) -> UiResult<PortfolioAnalytics<'a>> {
        Ok(PortfolioAnalytics::new(store, &self.portfolio)?)
    }

    /// Whether the scope already covers every account, so a second pass would repeat work.
    pub fn is_whole(&self, accounts: &[Account]) -> bool {
        accounts.iter().all(|a| self.accounts.contains(&a.id))
    }
}

/// What a background thread needs to open the profile's database on a connection of its own.
#[derive(Clone)]
pub struct DbAccess {
    pub path: PathBuf,
    key: Option<Zeroizing<[u8; 32]>>,
}

impl DbAccess {
    pub fn open(&self) -> sq_core::Result<Store> {
        match &self.key {
            Some(key) => Store::open_encrypted(&self.path, key),
            None => Store::open(&self.path),
        }
    }
}

/// What opening a profile produced: its database, portfolio and vault — or, while it is locked,
/// an empty in-memory stand-in that `store()` never hands out.
pub(crate) struct Opened {
    store: Store,
    portfolio: Portfolio,
    vault: Option<crate::vault::Unlocked>,
    locked: bool,
    remembered: bool,
}

/// Opens a profile as far as it opens without a password: fully when it has none or this device
/// remembers it, otherwise locked — its database is encrypted and there is no key to read it.
pub(crate) fn open_parts(id: &str, db_path: &std::path::Path) -> UiResult<Opened> {
    let folder = crate::profiles::folder_of(db_path);
    if !crate::vault::is_protected(folder) {
        let store = crate::dbfile::open(db_path, None)?;
        return Ok(Opened {
            portfolio: load_or_create_portfolio(&store)?,
            store,
            vault: None,
            locked: false,
            remembered: false,
        });
    }
    // A keychain that refuses, or a key that no longer opens the vault, is not an error here:
    // the profile simply opens locked and asks for its password.
    if let Ok(Some(data_key)) = crate::vault::recall(id)
        && let Ok(mut open) = crate::vault::unlock_with(folder, id, data_key)
    {
        let key = open.ensure_db_key()?;
        let store = crate::dbfile::open(db_path, Some(&key))?;
        return Ok(Opened {
            portfolio: load_or_create_portfolio(&store)?,
            store,
            vault: Some(open),
            locked: false,
            remembered: true,
        });
    }
    let store = Store::open_in_memory()?;
    Ok(Opened {
        portfolio: load_or_create_portfolio(&store)?,
        store,
        vault: None,
        locked: true,
        remembered: false,
    })
}

/// State shared by Tauri commands and background jobs.
pub struct AppState {
    store: Mutex<Store>,
    portfolio: Mutex<Portfolio>,
    /// The open profile's database; its settings and layouts sit beside it. Changes only in
    /// `open_profile`, so a background job copies it once and keeps writing where it started.
    db_path: Mutex<PathBuf>,
    /// See `db_in_use`.
    db_gate: RwLock<()>,
    pub profiles: crate::profiles::Profiles,
    /// Id of the open profile.
    profile: Mutex<String>,
    /// The open profile's vault, when it has a password and is unlocked (`secrets.rs`).
    pub(crate) vault: Mutex<Option<crate::vault::Unlocked>>,
    /// A protected profile not yet unlocked: `store()` refuses until it is.
    pub(crate) locked: AtomicBool,
    /// Whether this device keeps the open profile's data key in the keychain.
    pub(crate) remembered: AtomicBool,
    /// Application settings and refresh state.
    settings: Mutex<crate::settings::AppSettings>,
    /// Selected data scope.
    scope: Mutex<crate::scope::DataScope>,
    refresh: Mutex<crate::jobs::RefreshStatus>,
    /// Cancellation flag checked between network requests.
    pub cancel_refresh: AtomicBool,
    /// The same, for a running AI turn: checked between wire events, between agent steps, and
    /// while a consent card waits for an answer.
    pub ai_cancel: AtomicBool,
    /// Tool calls waiting on the user. The decision is resolved out of here by `request_id`,
    /// never from what the frontend echoes back — see `ai/consent.rs`.
    pub ai_consent: crate::ai::consent::Pending,
    /// What each provider last said it offers, keyed by provider id. In memory rather than
    /// `settings.json`: the list is cheap to refetch once per run, and a cached one on disk is a
    /// list to go stale. Keyed because a chat switched to another provider asks a second
    /// catalogue, and one slot would answer it with the first provider's models.
    pub ai_models: Mutex<HashMap<String, Vec<String>>>,
    /// What the app is extended with. Beside the profiles rather than inside one, so a theme
    /// survives switching profile (ADR-0070).
    pub plugins: crate::plugins::Plugins,
    /// Bytes currently being processed by the import wizard.
    import_file: Mutex<Option<(crate::commands::import::LoadedFile, Vec<u8>)>>,
}

impl AppState {
    /// Initialize paths, storage, settings, and the current portfolio.
    pub fn bootstrap(app: &App) -> UiResult<Self> {
        let dir = app.path().app_data_dir()?;
        std::fs::create_dir_all(&dir).map_err(|e| UiError::internal(e.to_string()))?;
        let profiles = crate::profiles::Profiles::new(&dir);
        // The first profile's name is user data, seeded in English like the default taxonomies.
        let profile = profiles.ensure("Default")?;
        let db_path = profiles.db_path(&profile.id);
        std::fs::create_dir_all(crate::profiles::folder_of(&db_path))
            .map_err(|e| UiError::internal(e.to_string()))?;

        let opened = open_parts(&profile.id, &db_path)?;
        let settings = crate::settings::load(&db_path);
        Ok(AppState {
            scope: Mutex::new(settings.scope.clone()),
            settings: Mutex::new(settings),
            refresh: Mutex::new(crate::jobs::RefreshStatus::default()),
            cancel_refresh: AtomicBool::new(false),
            ai_cancel: AtomicBool::new(false),
            ai_consent: crate::ai::consent::Pending::default(),
            ai_models: Mutex::new(HashMap::new()),
            store: Mutex::new(opened.store),
            portfolio: Mutex::new(opened.portfolio),
            db_path: Mutex::new(db_path),
            db_gate: RwLock::new(()),
            plugins: crate::plugins::Plugins::new(&dir),
            profiles,
            profile: Mutex::new(profile.id),
            vault: Mutex::new(opened.vault),
            locked: AtomicBool::new(opened.locked),
            remembered: AtomicBool::new(opened.remembered),
            import_file: Mutex::new(None),
        })
    }

    pub fn db_path(&self) -> UiResult<PathBuf> {
        self.db_path
            .lock()
            .map(|path| path.clone())
            .map_err(|_| UiError::internal("the profile state is poisoned"))
    }

    /// The database as a background thread opens it for itself: path and, for an encrypted
    /// profile, its key. A locked profile has none to give.
    pub fn db_access(&self) -> UiResult<DbAccess> {
        if self.is_locked() {
            return Err(UiError::Locked {
                message: "the profile is locked".into(),
            });
        }
        Ok(DbAccess {
            path: self.db_path()?,
            key: self.db_key()?,
        })
    }

    /// Held by every connection opened beside the main one, for as long as it is open. A file
    /// conversion (`dbfile::convert`) needs to be the only connection, so it takes this for
    /// writing and answers `busy` rather than wait.
    pub fn db_in_use(&self) -> RwLockReadGuard<'_, ()> {
        self.db_gate
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub(crate) fn db_exclusive(&self) -> UiResult<RwLockWriteGuard<'_, ()>> {
        self.db_gate.try_write().map_err(|_| UiError::Busy {
            message: "a refresh or an AI answer is using the database".into(),
        })
    }

    pub fn profile_id(&self) -> UiResult<String> {
        self.profile
            .lock()
            .map(|id| id.clone())
            .map_err(|_| UiError::internal("the profile state is poisoned"))
    }

    /// Replaces everything this state holds for one profile with another's. The new database is
    /// opened before anything is touched, so a profile that cannot be opened leaves the current
    /// one in place. A running refresh or AI turn is cancelled: each works on its own `Store`
    /// and finishes into the profile it started in.
    pub fn open_profile(&self, id: &str) -> UiResult<()> {
        self.profiles.find(id)?;
        let db_path = self.profiles.db_path(id);
        std::fs::create_dir_all(crate::profiles::folder_of(&db_path))
            .map_err(|e| UiError::internal(e.to_string()))?;
        let opened = open_parts(id, &db_path)?;
        let settings = crate::settings::load(&db_path);

        self.cancel_refresh.store(true, Ordering::Relaxed);
        self.ai_cancel.store(true, Ordering::Relaxed);
        self.ai_consent.clear();

        // Swapped under the store lock, which every command takes first: none of them sees one
        // profile's database beside another's portfolio. Taken raw — the profile being left may
        // itself be locked.
        let mut open = self.store_raw()?;
        self.locked.store(opened.locked, Ordering::Relaxed);
        self.remembered.store(opened.remembered, Ordering::Relaxed);
        *open = opened.store;
        *self.vault()? = opened.vault;
        *self.portfolio()? = opened.portfolio;
        *self.scope()? = settings.scope.clone();
        *self.settings()? = settings;
        *self
            .db_path
            .lock()
            .map_err(|_| UiError::internal("the profile state is poisoned"))? = db_path;
        *self
            .profile
            .lock()
            .map_err(|_| UiError::internal("the profile state is poisoned"))? = id.to_string();
        *self.import_file()? = None;
        // The custom provider's address is a setting, and settings are per profile.
        self.clear_models();
        drop(open);

        self.profiles.set_last(id)
    }

    /// The store whatever the lock says: for the code that swaps it.
    pub(crate) fn store_raw(&self) -> UiResult<MutexGuard<'_, Store>> {
        self.store
            .lock()
            .map_err(|_| UiError::internal("the database state is poisoned"))
    }

    /// Puts a freshly opened database in place of the current one and reads its portfolio.
    pub(crate) fn install(&self, store: Store) -> UiResult<()> {
        let portfolio = load_or_create_portfolio(&store)?;
        let mut open = self.store_raw()?;
        *open = store;
        *self.portfolio()? = portfolio;
        Ok(())
    }

    /// Lock storage, converting a poisoned mutex into a host error. A locked profile is refused
    /// here, the one door nearly every command passes through (ADR-0048).
    pub fn store(&self) -> UiResult<MutexGuard<'_, Store>> {
        if self.is_locked() {
            return Err(UiError::Locked {
                message: "the profile is locked".into(),
            });
        }
        self.store_raw()
    }

    pub fn portfolio(&self) -> UiResult<MutexGuard<'_, Portfolio>> {
        self.portfolio
            .lock()
            .map_err(|_| UiError::internal("the portfolio state is poisoned"))
    }

    pub fn scope(&self) -> UiResult<MutexGuard<'_, crate::scope::DataScope>> {
        self.scope
            .lock()
            .map_err(|_| UiError::internal("the scope state is poisoned"))
    }

    /// Return a portfolio copy narrowed to the active scope.
    pub fn scoped_portfolio(&self, store: &Store) -> UiResult<Portfolio> {
        let scope = self.scope()?.clone();
        let portfolio = self.portfolio()?.clone();
        if scope.kind == crate::scope::ScopeKind::Portfolio {
            return Ok(portfolio);
        }
        let groups = store.list_account_groups()?;
        let accounts = store.list_accounts()?;
        Ok(scope.apply(&portfolio, &groups, &accounts))
    }

    /// Return the full portfolio and the accounts selected for analytics.
    pub fn scope_selection(&self, store: &Store) -> UiResult<ScopeSelection> {
        self.scope_selection_in(store, None)
    }

    /// The same, in a scope the request names for itself. The picker is the app's lens and
    /// stays the default; a dashboard widget is allowed one of its own, so a board can hold a
    /// tile per account without the screen around it changing.
    pub fn scope_selection_in(
        &self,
        store: &Store,
        source: Option<&crate::scope::DataScope>,
    ) -> UiResult<ScopeSelection> {
        let portfolio = self.portfolio()?.clone();
        let accounts = match source {
            None => self.scoped_portfolio(store)?.account_ids,
            Some(scope) if scope.kind == crate::scope::ScopeKind::Portfolio => portfolio.account_ids.clone(),
            Some(scope) => {
                let groups = store.list_account_groups()?;
                let accounts = store.list_accounts()?;
                scope.apply(&portfolio, &groups, &accounts).account_ids
            }
        };
        Ok(ScopeSelection { portfolio, accounts })
    }

    /// Persist settings together with the active scope.
    pub fn persist_settings(&self) -> UiResult<()> {
        let mut settings = self.settings()?.clone();
        settings.scope = self.scope()?.clone();
        crate::settings::store(&self.db_path()?, &settings)?;
        *self.settings()? = settings;
        Ok(())
    }

    pub fn settings(&self) -> UiResult<MutexGuard<'_, crate::settings::AppSettings>> {
        self.settings
            .lock()
            .map_err(|_| UiError::internal("the settings state is poisoned"))
    }

    pub fn refresh(&self) -> UiResult<MutexGuard<'_, crate::jobs::RefreshStatus>> {
        self.refresh
            .lock()
            .map_err(|_| UiError::internal("the refresh state is poisoned"))
    }

    #[allow(clippy::type_complexity)]
    pub fn import_file(
        &self,
    ) -> UiResult<MutexGuard<'_, Option<(crate::commands::import::LoadedFile, Vec<u8>)>>> {
        self.import_file
            .lock()
            .map_err(|_| UiError::internal("the import state is poisoned"))
    }

    /// Reload the portfolio after account or currency changes.
    pub fn reload_portfolio(&self) -> UiResult<()> {
        let store = self.store()?;
        let fresh = load_or_create_portfolio(&store)?;
        *self.portfolio()? = fresh;
        Ok(())
    }
}

/// Load the first portfolio, creating a default one when needed.
fn load_or_create_portfolio(store: &Store) -> UiResult<Portfolio> {
    if let Some(existing) = store.list_portfolios()?.into_iter().next() {
        return Ok(existing);
    }
    let portfolio = Portfolio::new("Main", "EUR");
    store.save_portfolio(&portfolio)?;
    Ok(portfolio)
}
