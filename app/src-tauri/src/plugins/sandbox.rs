//! The one sandbox every compute plugin runs in (ADR-0086, ADR-0080). A reader and a writer are two
//! contracts over the same grant, and the grant is written once, here.
//!
//! There is no filesystem, no preopened directory, no reachable address and no real clock; what
//! WASI is linked for at all is that a guest carrying a language runtime will not instantiate
//! without `wasi:clocks` and `wasi:random`, and those two are handed over frozen and seeded — so a
//! plugin is not merely denied the time, it is unable to answer differently twice.

use crate::error::{UiError, UiResult};
use rand::SeedableRng;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::Duration;
use std::time::SystemTime;
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

/// How many compiled modules are kept. Compiling is most of what a call costs — a tenth of a
/// second for a small Rust guest, seconds for one carrying a language runtime — and a chat asks
/// the same tool turn after turn, so the last few are kept rather than every one ever run.
const KEPT: usize = 16;

struct Compiled {
    path: PathBuf,
    /// Size and modification time: a module edited in place is compiled again.
    stamp: (u64, Option<SystemTime>),
    component: Component,
}

fn compiled() -> MutexGuard<'static, Vec<Compiled>> {
    static COMPILED: Mutex<Vec<Compiled>> = Mutex::new(Vec::new());
    // A panic while holding it left at worst a stale list, which the stamp check still guards.
    COMPILED.lock().unwrap_or_else(|e| e.into_inner())
}

/// The module at `path`, compiled once per version of the file.
fn component(engine: &Engine, path: &Path) -> Result<Component, String> {
    let meta = std::fs::metadata(path).map_err(|e| format!("not a readable plugin module: {e}"))?;
    let stamp = (meta.len(), meta.modified().ok());
    if let Some(hit) = compiled().iter().find(|c| c.path == path && c.stamp == stamp) {
        return Ok(hit.component.clone());
    }
    // Compiled without the lock: another plugin's call must not wait for this one's compiler.
    let component =
        Component::from_file(engine, path).map_err(|e| format!("not a readable plugin module: {e}"))?;
    #[cfg(test)]
    tests::compiles().push(path.to_path_buf());
    let mut cache = compiled();
    cache.retain(|c| c.path != path);
    cache.push(Compiled {
        path: path.to_path_buf(),
        stamp,
        component: component.clone(),
    });
    if cache.len() > KEPT {
        cache.remove(0);
    }
    Ok(component)
}

/// Drops every compiled module under `folder`. Installing and removing call it, so a package
/// replaced by one whose file kept its size and time is never answered by the old code.
pub fn forget(folder: &Path) {
    compiled().retain(|c| !c.path.starts_with(folder));
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
    let component = component(engine, module)?;

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
    let _running = Running::start(engine, &mut store, DEADLINE);
    call(&mut store, &component, &linker)
}

/// How often the shared epoch moves while anything runs: the grain of every deadline.
const TICK: Duration = Duration::from_millis(100);

/// Calls in flight. The ticker sleeps while this is zero, so an idle app — a phone in a pocket —
/// does not wake ten times a second for nothing.
static RUNNING: AtomicUsize = AtomicUsize::new(0);

/// The one thread that moves the engine's epoch. The epoch is the *engine's*, shared by every
/// store, so a deadline is a number of ticks from where the epoch stood when that call began —
/// never a bump of its own, which would have ended every other plugin's call along with it.
fn ticker(engine: &Engine) -> &'static std::thread::Thread {
    static TICKER: OnceLock<std::thread::Thread> = OnceLock::new();
    TICKER.get_or_init(|| {
        let engine = engine.clone();
        std::thread::Builder::new()
            .name("plugin-epoch".into())
            .spawn(move || {
                loop {
                    if RUNNING.load(Ordering::Acquire) == 0 {
                        // An `unpark` that came first makes this return at once, so a call
                        // starting between the load and the park is not missed.
                        std::thread::park();
                        continue;
                    }
                    std::thread::sleep(TICK);
                    engine.increment_epoch();
                }
            })
            .expect("the plugin epoch thread starts")
            .thread()
            .clone()
    })
}

/// One call in flight: counted while it lives, and given its deadline in ticks.
struct Running;

impl Running {
    fn start(engine: &Engine, store: &mut Store<Host>, deadline: Duration) -> Running {
        let ticks = (deadline.as_millis() / TICK.as_millis()).max(1) as u64;
        store.set_epoch_deadline(ticks);
        RUNNING.fetch_add(1, Ordering::AcqRel);
        ticker(engine).unpark();
        Running
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        RUNNING.fetch_sub(1, Ordering::AcqRel);
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    /// Every compile this process made, by path — a test reads only its own paths, so tests
    /// running side by side do not see each other's.
    pub(super) fn compiles() -> MutexGuard<'static, Vec<PathBuf>> {
        static COMPILES: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
        COMPILES.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn compiled_count(path: &Path) -> usize {
        compiles().iter().filter(|p| *p == path).count()
    }

    fn start(path: &Path) {
        run(path, |store, component, linker| {
            linker
                .instantiate(&mut *store, component)
                .map(|_| ())
                .map_err(|e| e.to_string())
        })
        .unwrap();
    }

    /// `(module (func (export "spin") (loop (br 0))))`, assembled by hand: a stranger's loop.
    const SPIN: &[u8] = &[
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, // magic, version
        0x01, 0x04, 0x01, 0x60, 0x00, 0x00, // type: () -> ()
        0x03, 0x02, 0x01, 0x00, // one function of that type
        0x07, 0x08, 0x01, 0x04, b's', b'p', b'i', b'n', 0x00, 0x00, // export "spin"
        0x0a, 0x09, 0x01, 0x07, 0x00, 0x03, 0x40, 0x0c, 0x00, 0x0b, 0x0b, // loop { br 0 }
    ];

    /// Spins until its deadline, returning how long that took.
    fn spin(deadline: Duration) -> Duration {
        let engine = engine().unwrap();
        let module = wasmtime::Module::new(engine, SPIN).unwrap();
        let host = Host {
            table: ResourceTable::new(),
            wasi: WasiCtxBuilder::new().build(),
            limits: StoreLimitsBuilder::new().build(),
        };
        let mut store = Store::new(engine, host);
        let started = std::time::Instant::now();
        let _running = Running::start(engine, &mut store, deadline);
        let instance = wasmtime::Instance::new(&mut store, &module, &[]).unwrap();
        let spin = instance.get_typed_func::<(), ()>(&mut store, "spin").unwrap();
        assert!(spin.call(&mut store, ()).is_err(), "a loop ends at its deadline");
        started.elapsed()
    }

    /// The epoch is shared by every store, so a call that ran out of time must not take another
    /// one with it — which is what bumping the epoch as the deadline itself used to do.
    #[test]
    fn one_call_running_out_of_time_leaves_another_running() {
        let long = std::thread::spawn(|| spin(Duration::from_millis(2_000)));
        std::thread::sleep(Duration::from_millis(50));
        let short = spin(Duration::from_millis(300));
        assert!(
            short < Duration::from_millis(1_500),
            "the short call ended: {short:?}"
        );
        let long = long.join().unwrap();
        assert!(
            long >= Duration::from_millis(1_800),
            "the long call kept its own deadline: {long:?}"
        );
    }

    #[test]
    fn a_module_is_compiled_once_until_it_changes_or_is_forgotten() {
        let example =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/concentration/tool.wasm");
        let folder = std::env::temp_dir().join(format!("stonqs-sandbox-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&folder).unwrap();
        let module = folder.join("tool.wasm");
        std::fs::copy(&example, &module).unwrap();

        start(&module);
        start(&module);
        assert_eq!(
            compiled_count(&module),
            1,
            "the second call reuses the first compile"
        );

        forget(&folder);
        start(&module);
        assert_eq!(compiled_count(&module), 2, "a forgotten folder is compiled again");

        // Rewritten in place with a later time: another version of the file.
        let later = SystemTime::now() + std::time::Duration::from_secs(60);
        std::fs::File::options()
            .write(true)
            .open(&module)
            .unwrap()
            .set_modified(later)
            .unwrap();
        start(&module);
        assert_eq!(
            compiled_count(&module),
            3,
            "an edited module is not answered by the old one"
        );

        std::fs::remove_dir_all(&folder).ok();
    }
}
