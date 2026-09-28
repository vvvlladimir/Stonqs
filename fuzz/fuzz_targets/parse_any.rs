//! The reader the wizard calls on whatever file was picked. It decides for itself whether the
//! bytes are a canonical document, a Flex statement or a delimited file, so this one target
//! covers all three entry points the way the app reaches them.
#![no_main]

use libfuzzer_sys::fuzz_target;
use sq_core::import::{ParseConfig, parse_file};

fuzz_target!(|data: &[u8]| {
    let _ = parse_file(data, &ParseConfig::default());
});
