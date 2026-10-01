//! Body schemas for esign and calibration operations.

use super::{SchemaBinding, SchemaMap, merge_type, schema_ref};
use crate::handlers::esign::{CalibrationApprovedBody, EsignMintBody};
use serde_json::Value;
use wicket_esign::{ArchivalBundle, Challenge, Manifestation};

pub fn register(map: &mut SchemaMap) {
    map.insert(
        "approveCalibration",
        SchemaBinding {
            request: None,
            response: schema_ref::<CalibrationApprovedBody>(),
        },
    );
    map.insert(
        "esignChallenge",
        SchemaBinding {
            request: None,
            response: schema_ref::<Challenge>(),
        },
    );
    map.insert(
        "esignMint",
        SchemaBinding {
            request: Some(schema_ref::<EsignMintBody>()),
            response: schema_ref::<Manifestation>(),
        },
    );
    map.insert(
        "getEsignSignature",
        SchemaBinding {
            request: None,
            response: schema_ref::<Manifestation>(),
        },
    );
    map.insert(
        "getEsignBundle",
        SchemaBinding {
            request: None,
            response: schema_ref::<ArchivalBundle>(),
        },
    );
}

pub fn merge_components(schemas: &mut serde_json::Map<String, Value>) {
    merge_type::<CalibrationApprovedBody>(schemas);
    merge_type::<Challenge>(schemas);
    merge_type::<EsignMintBody>(schemas);
    merge_type::<Manifestation>(schemas);
    merge_type::<ArchivalBundle>(schemas);
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn merged_components_describe_the_wire() {
        let mut schemas = serde_json::Map::new();
        super::merge_components(&mut schemas);
        let sig = &schemas["Manifestation"]["properties"]["signature"];
        assert_eq!(
            sig["allOf"][0]["$ref"], "#/components/schemas/SignatureManifest",
            "{sig}"
        );
        assert_eq!(
            schemas["SignatureManifest"]["properties"]["record_content_hash"]["type"],
            "string"
        );
        let hash = &schemas["ArchivalBundle"]["properties"]["record_content_hash"];
        assert_eq!(hash["type"], "array", "{hash}");
        assert_eq!(hash["minItems"], 32);
        assert_eq!(hash["maxItems"], 32);
        let snapshot = &schemas["ArchivalBundle"]["properties"]["record_snapshot"];
        assert!(
            snapshot.is_object() && snapshot != &json!(true),
            "{snapshot}"
        );
        assert_eq!(
            schemas["CalibrationApprovedBody"]["properties"]["status"]["enum"],
            json!(["approved"])
        );
        assert!(schemas["EsignRecordBody"].is_object());
        assert!(schemas["EsignIdentBody"].is_object());
        assert!(schemas["SealRef"]["properties"]["hash"]["type"] == "array");
        assert!(schemas["AnchorRef"].is_object());
        assert!(schemas["ManifestRecord"].is_object());
        assert!(schemas["EsignMintBody"]["properties"]["meaning"].is_object());
        assert!(schemas["Challenge"]["properties"]["components_required"].is_object());
    }
}
