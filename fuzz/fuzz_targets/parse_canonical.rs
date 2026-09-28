//! The app's own transaction file (ADR-0066). It is read back from an export the user keeps, so
//! a truncated or hand-edited copy must refuse rather than come apart.
#![no_main]

use libfuzzer_sys::fuzz_target;
use sq_core::import::parse_canonical;

fuzz_target!(|data: &[u8]| {
    let _ = parse_canonical(data);
});
