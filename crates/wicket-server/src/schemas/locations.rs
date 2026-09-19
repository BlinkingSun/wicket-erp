//! Body schemas for location operations.

use super::{SchemaBinding, SchemaMap, merge_type, schema_ref};
use crate::handlers::locations::{LocCreate, LocationSummary};
use serde_json::Value;
use wicket_mod_locations::{ListResponse, Location, LocationTreeNode};

pub fn register(map: &mut SchemaMap) {
    map.insert(
        "listLocations",
        SchemaBinding {
            request: None,
            response: schema_ref::<ListResponse<Location>>(),
        },
    );
    map.insert(
        "createLocation",
        SchemaBinding {
            request: Some(schema_ref::<LocCreate>()),
            response: schema_ref::<LocationSummary>(),
        },
    );
    map.insert(
        "getLocation",
        SchemaBinding {
            request: None,
            response: schema_ref::<LocationSummary>(),
        },
    );
    map.insert(
        "listLocationTree",
        SchemaBinding {
            request: None,
            response: schema_ref::<ListResponse<LocationTreeNode>>(),
        },
    );
    map.insert(
        "deactivateLocation",
        SchemaBinding {
            request: None,
            response: schema_ref::<Location>(),
        },
    );
}

pub fn merge_components(schemas: &mut serde_json::Map<String, Value>) {
    merge_type::<LocCreate>(schemas);
    merge_type::<LocationSummary>(schemas);
    merge_type::<Location>(schemas);
    merge_type::<LocationTreeNode>(schemas);
    merge_type::<ListResponse<Location>>(schemas);
    merge_type::<ListResponse<LocationTreeNode>>(schemas);
}

#[cfg(test)]
mod tests {
    use super::*;
    use schemars::JsonSchema;

    #[test]
    fn registers_five_location_ops() {
        let mut map = SchemaMap::new();
        register(&mut map);
        let ids: Vec<_> = map.keys().copied().collect();
        assert_eq!(
            ids,
            [
                "createLocation",
                "deactivateLocation",
                "getLocation",
                "listLocationTree",
                "listLocations",
            ]
        );
        assert!(map["createLocation"].request.is_some());
        assert!(map["deactivateLocation"].request.is_none());
        assert!(map["getLocation"].request.is_none());
        assert!(map["listLocationTree"].request.is_none());
        assert!(map["listLocations"].request.is_none());
    }

    #[test]
    fn schema_names_match_refs() {
        assert_eq!(LocCreate::schema_name(), "LocCreate");
        assert_eq!(LocationSummary::schema_name(), "LocationSummary");
        assert_eq!(Location::schema_name(), "Location");
        assert_eq!(LocationTreeNode::schema_name(), "LocationTreeNode");
        assert_eq!(
            ListResponse::<Location>::schema_name(),
            "ListResponse_for_Location"
        );
        assert_eq!(
            ListResponse::<LocationTreeNode>::schema_name(),
            "ListResponse_for_LocationTreeNode"
        );
    }
}
