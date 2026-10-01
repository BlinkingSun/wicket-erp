//! Location entities and validation (no I/O).

use schemars::JsonSchema;
use wicket_core::{Boundary, Identifier, LocationId};

/// Lifecycle status of a location row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LocationStatus {
    /// Usable for postings.
    Active,
    /// Hidden from pick lists; cannot receive new stock.
    Inactive,
}

impl LocationStatus {
    /// SQL label.
    pub fn as_sql(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Inactive => "inactive",
        }
    }

    /// Parse SQL label.
    pub fn from_sql(s: &str) -> Option<Self> {
        match s {
            "active" => Some(Self::Active),
            "inactive" => Some(Self::Inactive),
            _ => None,
        }
    }
}

/// Structural kind of a location node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LocationKind {
    /// Top-level warehouse.
    Warehouse,
    /// Area within a warehouse.
    Area,
    /// Storage bin.
    Bin,
    /// Work-order WIP place (real, valued).
    Wip,
    /// Outside-processing vendor place (real, valued).
    Osp,
    /// Ledger virtual boundary location.
    Virtual,
}

impl LocationKind {
    /// SQL label.
    pub fn as_sql(self) -> &'static str {
        match self {
            Self::Warehouse => "warehouse",
            Self::Area => "area",
            Self::Bin => "bin",
            Self::Wip => "wip",
            Self::Osp => "osp",
            Self::Virtual => "virtual",
        }
    }

    /// Parse SQL label.
    pub fn from_sql(s: &str) -> Option<Self> {
        match s {
            "warehouse" => Some(Self::Warehouse),
            "area" => Some(Self::Area),
            "bin" => Some(Self::Bin),
            "wip" => Some(Self::Wip),
            "osp" => Some(Self::Osp),
            "virtual" => Some(Self::Virtual),
            _ => None,
        }
    }
}

/// Site (plant / facility).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Site {
    /// Primary key.
    pub id: Identifier,
    /// Uppercase code.
    pub code: String,
    /// Display name.
    pub name: String,
    /// Optimistic version.
    pub version: i64,
}

/// Location master row.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, JsonSchema)]
pub struct Location {
    /// Primary key.
    pub id: LocationId,
    /// Unique uppercase code.
    pub code: String,
    /// Display name.
    pub name: String,
    /// Owning site.
    pub site_id: Identifier,
    /// Parent in the tree (`None` for roots).
    pub parent_id: Option<LocationId>,
    /// Structural kind.
    pub kind: LocationKind,
    /// Virtual boundary class when `kind == Virtual`.
    /// Serde is PascalCase variant name or null; `Boundary` has no `JsonSchema`.
    #[schemars(with = "Option<String>")]
    pub boundary_class: Option<Boundary>,
    /// Active flag.
    pub status: LocationStatus,
    /// Work order for WIP rows.
    pub work_order_id: Option<Identifier>,
    /// Optimistic version.
    pub version: i64,
}

/// Input to create a real (non-boundary) location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateLocation {
    /// Unique uppercase code.
    pub code: String,
    /// Display name.
    pub name: String,
    /// Site id.
    pub site_id: Identifier,
    /// Optional parent.
    pub parent_id: Option<LocationId>,
    /// Kind (not `Virtual`; boundaries are seeded).
    pub kind: LocationKind,
}

/// Cursor filter for `GET /api/v1/locations` (`docs/10` §2.3).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ListFilter {
    /// Page size. Default 50, maximum 200.
    pub limit: Option<u32>,
    /// Last id from the previous page (`id` ascending).
    pub cursor: Option<LocationId>,
}

/// Patchable fields (`boundary_class` is never accepted).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UpdateLocation {
    /// New name.
    pub name: Option<String>,
    /// Reparent within the tree.
    pub parent_id: Option<Option<LocationId>>,
    /// Version for optimistic concurrency.
    pub version: i64,
}

/// Tree node for hierarchical responses.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, JsonSchema)]
pub struct LocationTreeNode {
    /// This node.
    pub location: Location,
    /// Children in sort order by code.
    pub children: Vec<LocationTreeNode>,
}

/// Validate a location code (uppercase letters, digits, hyphen).
pub fn validate_code(code: &str) -> Result<(), crate::Error> {
    if code.is_empty() || code.len() > 64 {
        return Err(crate::Error::Validation("code length".into()));
    }
    if !code
        .bytes()
        .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err(crate::Error::Validation(
            "code must be uppercase A-Z, digits, or hyphen".into(),
        ));
    }
    Ok(())
}

/// Map a core [`Boundary`] to the seeded virtual location code.
pub fn boundary_code(boundary: Boundary) -> &'static str {
    match boundary {
        Boundary::Supplier => "BOUNDARY-SUPPLIER",
        Boundary::Customer => "BOUNDARY-CUSTOMER",
        Boundary::Scrap => "BOUNDARY-SCRAP",
        Boundary::Adjustment => "BOUNDARY-ADJUSTMENT",
        Boundary::Rounding => "BOUNDARY-ROUNDING",
        Boundary::Consumed => "BOUNDARY-CONSUMED",
        Boundary::Produced => "BOUNDARY-PRODUCED",
        _ => "BOUNDARY-UNKNOWN",
    }
}

/// All seven ledger virtual boundaries seeded at install.
pub const BOUNDARY_VARIANTS: [Boundary; 7] = [
    Boundary::Supplier,
    Boundary::Customer,
    Boundary::Scrap,
    Boundary::Adjustment,
    Boundary::Rounding,
    Boundary::Consumed,
    Boundary::Produced,
];

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn sample() -> Location {
        Location {
            id: LocationId::from_uuid(uuid::Uuid::nil()),
            code: "WH-1".into(),
            name: "Main".into(),
            site_id: Identifier::from_uuid(uuid::Uuid::nil()),
            parent_id: None,
            kind: LocationKind::Warehouse,
            boundary_class: None,
            status: LocationStatus::Active,
            work_order_id: None,
            version: 1,
        }
    }

    #[test]
    fn location_serde_keys_and_nulls() {
        let v = serde_json::to_value(sample()).expect("json");
        let obj = v.as_object().expect("object");
        let mut keys: Vec<&str> = obj.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "boundary_class",
                "code",
                "id",
                "kind",
                "name",
                "parent_id",
                "site_id",
                "status",
                "version",
                "work_order_id",
            ]
        );
        assert_eq!(v["kind"], "warehouse");
        assert_eq!(v["status"], "active");
        assert_eq!(v["boundary_class"], Value::Null);
        assert_eq!(v["parent_id"], Value::Null);
        assert_eq!(v["work_order_id"], Value::Null);
        assert_eq!(v["id"], uuid::Uuid::nil().to_string());
    }

    #[test]
    fn location_kind_as_sql_matches_serde() {
        for kind in [
            LocationKind::Warehouse,
            LocationKind::Area,
            LocationKind::Bin,
            LocationKind::Wip,
            LocationKind::Osp,
            LocationKind::Virtual,
        ] {
            let ser = serde_json::to_value(kind).expect("kind json");
            assert_eq!(ser, kind.as_sql());
        }
    }

    #[test]
    fn boundary_class_serde_is_pascal_case() {
        let mut loc = sample();
        loc.kind = LocationKind::Virtual;
        loc.boundary_class = Some(Boundary::Supplier);
        let v = serde_json::to_value(&loc).expect("json");
        assert_eq!(v["boundary_class"], "Supplier");
    }
}
