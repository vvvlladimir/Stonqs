//! The one sandbox every compute plugin runs in (ADR-0073, ADR-0080). A reader and a writer are two
//! contracts over the same grant, and the grant is written once, here.
//!
//! There is no filesystem, no preopened directory, no reachable address and no real clock; what
//! WASI is linked for at all is that a guest carrying a language runtime will not instantiate
//! without `wasi:clocks` and `wasi:random`, and those two are handed over frozen and seeded — so a
//! plugin is not merely denied the time, it is unable to answer differently twice.

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

/// How long one call may take. Generous, because it is an interpreter on a phone and a statement
/// can be long; finite, because a stranger's loop must fail the call rather than hang the app.
const DEADLINE: Duration = Duration::from_secs(20);

/// What a plugin may allocate. A broker file is measured in megabytes, and everything the plugin
/// builds out of it lives in this same allowance.
const MEMORY: usize = 256 * 1024 * 1024;

/// The moment every plugin is told it is. Not today's date: a plugin that dated a row from the
/// clock would produce a file that changed under the user.
const FROZEN: Duration = Duration::from_secs(0);

/// The seed every plugin is given. One constant, because the point is that there is no entropy
/// here at all, not that each run has its own.
const SEED: u64 = 0x5109_1173;

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

pub struct Host {
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

/// Loads `module` and hands `call` a store, the component and a linker holding nothing but the
/// stubbed WASI. Whatever goes wrong on this side — a file that is not a component, a trap, the
/// deadline, the memory ceiling — is one `Err(String)`: from the caller's side they are one thing,
/// which is that this plugin could not do it.
pub fn run<T>(
    module: &Path,
    call: impl FnOnce(&mut Store<Host>, &Component, &Linker<Host>) -> Result<T, String>,
) -> Result<T, String> {
    let engine = engine().map_err(|e| format!("{e:?}"))?;
    let component =
        Component::from_file(engine, module).map_err(|e| format!("not a readable plugin module: {e}"))?;

    let mut linker: Linker<Host> = Linker::new(engine);
    add_to_linker_sync(&mut linker).map_err(|e| e.to_string())?;

    let mut wasi = WasiCtxBuilder::new();
    wasi.wall_clock(Frozen)
        .monotonic_clock(Frozen)
        .secure_random(rand::rngs::StdRng::seed_from_u64(SEED))
        .insecure_random(rand::rngs::StdRng::seed_from_u64(SEED))
        .insecure_random_seed(u128::from(SEED));
    // Everything a `WasiCtxBuilder` is not told stays off: no preopened directory, no inherited
    // standard input, and an address list that is empty. Output is swallowed — a plugin says what
    // it has to say in what it returns, not onto a console nobody reads.
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

    let outcome = call(&mut store, &component, &linker);
    drop(finished);
    outcome
}
