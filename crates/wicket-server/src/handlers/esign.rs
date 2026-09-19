//! Esign and calibration HTTP handlers.
//!
//! Mint, manifestation, bundle, challenge, and calibration stay in `mod.rs`
//! because `crates/wicket-server/tests/esign.rs` include_str-pins those
//! function bodies to that file, and this lane does not own that test.
