#![no_std]

pub mod debouncer;
pub mod status;
pub mod toggle;

pub(crate) fn to_bincode_config() -> impl bincode::config::Config {
    bincode::config::standard()
        .with_big_endian()
        .with_fixed_int_encoding()
}
