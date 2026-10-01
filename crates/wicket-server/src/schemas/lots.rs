//! Body schemas for lot and serial operations.

use super::{SchemaBinding, SchemaMap, merge_type, schema_ref};
use crate::envelope::ListBody;
use crate::handlers::lots::{PackageCreate, PackageCreatedBody, PackageListItem};
use serde_json::Value;
use wicket_mod_lots::{CreateLotBody, CreateSerialsBody, LotBody, SerialBody, SetStatusBody};

pub fn register(map: &mut SchemaMap) {
    map.insert(
        "getLot",
        SchemaBinding {
            request: None,
            response: schema_ref::<LotBody>(),
        },
    );
    map.insert(
        "listSerials",
        SchemaBinding {
            request: None,
            response: schema_ref::<ListBody<SerialBody>>(),
        },
    );
    map.insert(
        "listLots",
        SchemaBinding {
            request: None,
            response: schema_ref::<wicket_mod_lots::ListBody<LotBody>>(),
        },
    );
    map.insert(
        "createLot",
        SchemaBinding {
            request: Some(schema_ref::<CreateLotBody>()),
            response: schema_ref::<LotBody>(),
        },
    );
    map.insert(
        "setLotStatus",
        SchemaBinding {
            request: Some(schema_ref::<SetStatusBody>()),
            response: schema_ref::<LotBody>(),
        },
    );
    map.insert(
        "listPackages",
        SchemaBinding {
            request: None,
            response: schema_ref::<ListBody<PackageListItem>>(),
        },
    );
    map.insert(
        "createPackage",
        SchemaBinding {
            request: Some(schema_ref::<PackageCreate>()),
            response: schema_ref::<PackageCreatedBody>(),
        },
    );
    map.insert(
        "createSerials",
        SchemaBinding {
            request: Some(schema_ref::<CreateSerialsBody>()),
            response: schema_ref::<wicket_mod_lots::ListBody<SerialBody>>(),
        },
    );
}

pub fn merge_components(schemas: &mut serde_json::Map<String, Value>) {
    merge_type::<LotBody>(schemas);
    merge_type::<SerialBody>(schemas);
    merge_type::<ListBody<SerialBody>>(schemas);
    merge_type::<wicket_mod_lots::ListBody<LotBody>>(schemas);
    merge_type::<wicket_mod_lots::ListBody<SerialBody>>(schemas);
    merge_type::<CreateLotBody>(schemas);
    merge_type::<SetStatusBody>(schemas);
    merge_type::<CreateSerialsBody>(schemas);
    merge_type::<PackageCreate>(schemas);
    merge_type::<PackageCreatedBody>(schemas);
    merge_type::<PackageListItem>(schemas);
    merge_type::<ListBody<PackageListItem>>(schemas);
}
