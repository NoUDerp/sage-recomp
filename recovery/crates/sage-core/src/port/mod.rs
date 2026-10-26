//! Rust ports of openbfme_core modules, one module per crowdsourced task.
//!
//! Each file is a standalone, safe, deterministic translation of one upstream
//! C++ module; the verifier compiles it on its own against the golden vectors
//! in `goldens/`. Files still carrying `unimplemented!()` are open tasks.

pub mod command_points;
pub mod det_libm_exact;
pub mod det_libm_transcendental;
pub mod dict;
pub mod ini_macro;
pub mod kind_of_tokens;
pub mod md5;
pub mod mini_json;
pub mod mp3_tables;
pub mod mp_game_rules;
pub mod name_key;
pub mod numeric_pc24;
pub mod path_io;
pub mod random_value;
pub mod sha256;
pub mod xfer_crc;
