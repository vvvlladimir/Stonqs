//! Interactive Brokers Flex XML (ADR-0061): attributes, entities and nesting, none of it written
//! by this project.
#![no_main]

use libfuzzer_sys::fuzz_target;
use sq_core::import::parse_flex;

fuzz_target!(|data: &[u8]| {
    let _ = parse_flex(data);
});
