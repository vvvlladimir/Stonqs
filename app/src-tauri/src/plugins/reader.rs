//! Running a plugin's file reader (ADR-0073): bytes in, a `stonqs.transactions` document out.
//!
//! The module is the whole of what this app grants a stranger's code. There is no filesystem, no
//! preopened directory, no reachable address and no real clock; what WASI is linked for at all is
//! that a guest carrying a language runtime will not instantiate without `wasi:clocks` and
//! `wasi:random`, and those two are handed over frozen and seeded — so a reader is not merely
//! denied the time, it is unable to answer differently twice.
//!
//! Nothing here touches the store or the portfolio. A reader is run once, by `import_load`, and
//! what it produced is what every later preview and the commit read.

use crate::error::{UiError, UiResult};
use rand::SeedableRng;
use std::path::Path;
use std::sync::OnceLock;
use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;
use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store, StoreLimits, StoreLimitsBuilder};
use wasmtime_wasi::clocks::{HostMonotonicClock, HostWallClock};
use wasmtime_wasi::p2::add_to_linker_sync;
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

/// The generated side of the contract, kept in a module of its own: the world's `reading` and
/// `warning` become Rust types with the names this file also wants for the host's own.
mod wit {
    wasmtime::component::bindgen!({
        path: "wit",
        world: "reader",
    });
}

use wit::{FileHints, ReadError};

/// How long a reader may take over one file. Generous, because it is an interpreter on a phone
/// and a statement can be long; finite, because a stranger's loop must fail the import rather
/// than hang the app.
const DEADLINE: Duration = Duration::from_secs(20);

/// What a reader may allocate. A broker file is measured in megabytes, and everything the reader
/// builds out of it lives in this same allowance.
const MEMORY: usize = 256 * 1024 * 1024;

/// The moment every reader is told it is. Not today's date: a reader that dated a row from the
/// clock would produce a file that changed under the user, and the one date a statement is about
/// is printed in the statement.
const FROZEN: Duration = Duration::from_secs(0);

/// The seed every reader is given. One constant, because the point is that there is no entropy
/// here at all, not that each run has its own.
const SEED: u64 = 0x5109_1173;

/// What a reader produced, in the host's own words.
#[derive(Debug, Clone)]
pub struct Reading {
    /// A `stonqs.transactions` document (ADR-0066), as the reader wrote it. Unvalidated here —
    /// `parse_canonical` is what reads it, exactly as it reads one the user brought themselves.
    pub canonical: String,
    pub warnings: Vec<ReaderWarning>,
}

/// A reader's own warning. Its `code` is a stranger's vocabulary, so it travels beside the
/// sentence rather than instead of it: the app has no table to translate it from.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ReaderWarning {
    /// Which plugin's reader said it.
    pub plugin: String,
    pub row: Option<u32>,
    pub code: String,
    pub message: String,
}

/// Why a reader did not produce a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// Not a file this reader knows. The only one that is not a failure: the host moves on.
    NotMine,
    /// Sealed, and the password was absent or wrong.
    NeedsPassword,
    /// Recognised and unreadable, or the module itself failed — trapped, timed out, ran out of
    /// memory, or was not a component at all. One case from the user's side: this plugin cannot
    /// read this file.
    Failed(String),
}

impl Refusal {
    pub fn into_error(self, plugin: &str) -> UiError {
        match self {
            Refusal::NotMine => UiError::invalid(format!("reader {plugin} does not read this file")),
            Refusal::NeedsPassword => UiError::FileProtected {
                message: format!("the file is protected and reader {plugin} needs its password"),
            },
            Refusal::Failed(why) => UiError::Reader {
                plugin: plugin.to_string(),
                message: why,
            },
        }
    }
}

/// Compiling a module needs one, and building one is the expensive part of this file — so it is
/// built once and shared. It holds no state of any guest: every run gets its own `Store`.
fn engine() -> UiResult<&'static Engine> {
    static ENGINE: OnceLock<Result<Engine, String>> = OnceLock::new();
    ENGINE
        .get_or_init(|| {
            let mut config = Config::new();
            // The deadline below is a thread bumping the epoch; without this the module would
            // never look at it.
            config.epoch_interruption(true);
            Engine::new(&config).map_err(|e| e.to_string())
        })
        .as_ref()
        .map_err(|e| UiError::internal(format!("the plugin runtime is unavailable: {e}")))
}

struct Host {
    table: ResourceTable,
    wasi: WasiCtx,
    limits: StoreLimits,
}

impl WasiView for Host {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

/// A clock that says the same thing every time it is asked.
struct Frozen;

impl HostWallClock for Frozen {
    fn resolution(&self) -> Duration {
        Duration::from_secs(1)
    }

    fn now(&self) -> Duration {
        FROZEN
    }
}

impl HostMonotonicClock for Frozen {
    fn resolution(&self) -> u64 {
        1
    }

    fn now(&self) -> u64 {
        0
    }
}

/// Runs one reader over one file.
///
/// `module` is a path inside the plugin's own folder, already checked by `safe_join`. Everything
/// that can go wrong on this side — a file that is not a component, a trap, the deadline, the
/// memory ceiling — comes back as `Refusal::Failed`: from where the import stands they are one
/// thing, which is that this plugin cannot read this file.
pub fn read(
    module: &Path,
    bytes: &[u8],
    file_name: &str,
    password: Option<&str>,
) -> Result<Reading, Refusal> {
    let engine = engine().map_err(|e| Refusal::Failed(format!("{e:?}")))?;
    let component = Component::from_file(engine, module)
        .map_err(|e| Refusal::Failed(format!("not a readable plugin module: {e}")))?;

    let mut linker: Linker<Host> = Linker::new(engine);
    add_to_linker_sync(&mut linker).map_err(|e| Refusal::Failed(e.to_string()))?;

    let mut wasi = WasiCtxBuilder::new();
    wasi.wall_clock(Frozen)
        .monotonic_clock(Frozen)
        .secure_random(rand::rngs::StdRng::seed_from_u64(SEED))
        .insecure_random(rand::rngs::StdRng::seed_from_u64(SEED))
        .insecure_random_seed(u128::from(SEED));
    // Everything a `WasiCtxBuilder` is not told stays off: no preopened directory, no inherited
    // standard input, and an address list that is empty. Output is swallowed — a reader writes
    // its findings into the warnings it returns, not onto a console nobody reads.
    let host = Host {
        table: ResourceTable::new(),
        wasi: wasi.build(),
        limits: StoreLimitsBuilder::new().memory_size(MEMORY).build(),
    };

    let mut store = Store::new(engine, host);
    store.limiter(|host| &mut host.limits);
    store.set_epoch_deadline(1);

    // One bump of the epoch is the deadline. The channel is how the thread learns the call
    // finished: a disconnect is not a timeout, so the two are told apart explicitly.
    let (finished, waiting) = std::sync::mpsc::channel::<()>();
    let ticker = engine.clone();
    std::thread::spawn(move || {
        if matches!(waiting.recv_timeout(DEADLINE), Err(RecvTimeoutError::Timeout)) {
            ticker.increment_epoch();
        }
    });

    let outcome = (|| {
        let reader = wit::Reader::instantiate(&mut store, &component, &linker)
            .map_err(|e| Refusal::Failed(format!("the module did not start: {e}")))?;
        let hints = FileHints {
            file_name: file_name.to_string(),
            password: password.map(str::to_string),
        };
        reader
            .call_read(&mut store, bytes, &hints)
            .map_err(|e| Refusal::Failed(format!("the module stopped: {e}")))
    })();
    drop(finished);

    match outcome? {
        Ok(reading) => Ok(Reading {
            canonical: reading.canonical,
            warnings: reading
                .warnings
                .into_iter()
                .map(|w| ReaderWarning {
                    plugin: String::new(),
                    row: w.row,
                    code: w.code,
                    message: w.message,
                })
                .collect(),
        }),
        Err(ReadError::NotMine) => Err(Refusal::NotMine),
        Err(ReadError::NeedsPassword) => Err(Refusal::NeedsPassword),
        Err(ReadError::Malformed(why)) => Err(Refusal::Failed(why)),
    }
}

/// What a reader must prove before the package carrying it is installed: that it recognises the
/// sample it ships, that what it produces is the app's own transaction file, and that the file is
/// the one the package says it is.
///
/// The last of the three is what a broker layout cannot be asked for and a reader must be: a
/// layout that misreads a column leaves a question in the wizard, while a reader that misreads one
/// hands over a document that looks perfectly correct.
pub fn check(id: &str, module: &Path, sample: &[u8], sample_name: &str, expected: &str) -> UiResult<()> {
    let reading = read(module, sample, sample_name, None).map_err(|refusal| match refusal {
        Refusal::NotMine => UiError::invalid(format!("reader {id} does not recognise the sample it ships")),
        other => other.into_error(id),
    })?;

    sq_core::import::parse_canonical(reading.canonical.as_bytes())
        .map_err(|e| UiError::invalid(format!("reader {id} produced no readable transaction file: {e}")))?;

    // Compared as documents rather than as text: how a reader spaces its JSON is not a promise
    // it made, and the operations are.
    let produced: serde_json::Value = serde_json::from_str(&reading.canonical)
        .map_err(|e| UiError::invalid(format!("reader {id}: {e}")))?;
    let wanted: serde_json::Value = serde_json::from_str(expected)
        .map_err(|e| UiError::invalid(format!("reader {id}: its own expectation is not readable: {e}")))?;
    if produced != wanted {
        return Err(UiError::invalid(format!(
            "reader {id} does not read its own sample the way the package says it does"
        )));
    }
    Ok(())
}
