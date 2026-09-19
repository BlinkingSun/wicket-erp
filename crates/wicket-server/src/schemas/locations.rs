//! Body schemas for location operations.
//!
//! Empty in this lane. The `ui1` schema lane registers here and does not
//! edit `schemas/mod.rs`.

use super::SchemaMap;
use serde_json::Value;

pub fn register(_map: &mut SchemaMap) {}

pub fn merge_components(_schemas: &mut serde_json::Map<String, Value>) {}
