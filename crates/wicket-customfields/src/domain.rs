//! Domain types for custom field definitions and values.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use wicket_core::Identifier;

/// Machine `doc_type` registered by [`crate::definition_machine`].
pub const DOC_TYPE: &str = "customfields.definition";

/// Permission keys exported for the composition root.
pub const PERMISSIONS: &[&str] = &["customfields.retire"];

/// Stable definition identity (all versions share this id).
///
/// Serde is the inner uuid string (newtype). The derived schema is that
/// string with `format: uuid`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct DefinitionId(Identifier);

impl DefinitionId {
    /// Wrap an identifier.
    pub fn new(id: Identifier) -> Self {
        Self(id)
    }

    /// Generate a new id.
    pub fn generate() -> Self {
        Self(Identifier::generate())
    }

    /// Underlying identifier.
    pub fn as_identifier(self) -> Identifier {
        self.0
    }

    /// Underlying uuid.
    pub fn as_uuid(self) -> uuid::Uuid {
        self.0.as_uuid()
    }
}

/// Field key (manifest / API).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FieldKey(pub String);

/// Stored field type (no JSON blob).
///
/// Serde emits the variant name (`"String"`), not [`Self::as_str`] (`"string"`).
/// List-definition JSON uses this enum; request bodies and [`ValueWire::type_name`]
/// use the `as_str` token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[non_exhaustive]
pub enum FieldType {
    /// Short string.
    String,
    /// Long text.
    Text,
    /// Integer.
    Integer,
    /// Decimal with scale.
    Decimal,
    /// Boolean.
    Bool,
    /// Calendar date with precision (inv 12).
    Date,
    /// Enumerated string.
    Enum,
    /// Reference to another entity record.
    Reference,
}

impl FieldType {
    /// Parse manifest / SQL type name.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "string" => Some(Self::String),
            "text" => Some(Self::Text),
            "integer" => Some(Self::Integer),
            "decimal" => Some(Self::Decimal),
            "bool" => Some(Self::Bool),
            "date" => Some(Self::Date),
            "enum" => Some(Self::Enum),
            "reference" => Some(Self::Reference),
            _ => None,
        }
    }

    /// SQL / manifest name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Text => "text",
            Self::Integer => "integer",
            Self::Decimal => "decimal",
            Self::Bool => "bool",
            Self::Date => "date",
            Self::Enum => "enum",
            Self::Reference => "reference",
        }
    }
}

/// Definition lifecycle.
///
/// Serde emits the variant name (`"Active"` / `"Retired"`), not [`Self::as_str`].
/// The retire HTTP blob sends the `as_str` token `"retired"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[non_exhaustive]
pub enum DefinitionStatus {
    /// Active definitions accept writes.
    Active,
    /// Retired definitions are read-only.
    Retired,
}

impl DefinitionStatus {
    /// SQL / machine state name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Retired => "retired",
        }
    }

    pub(crate) fn parse(s: &str) -> Option<Self> {
        match s {
            "active" => Some(Self::Active),
            "retired" => Some(Self::Retired),
            _ => None,
        }
    }
}

/// Date precision (inv 12).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum DatePrecision {
    /// Day precision.
    Day,
    /// Month precision (stored as first of month).
    Month,
    /// Year precision (stored as 1 January).
    Year,
}

impl DatePrecision {
    /// Wire / SQL name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Day => "day",
            Self::Month => "month",
            Self::Year => "year",
        }
    }

    pub(crate) fn parse(s: &str) -> Option<Self> {
        match s {
            "day" => Some(Self::Day),
            "month" => Some(Self::Month),
            "year" => Some(Self::Year),
            _ => None,
        }
    }
}

/// Effectivity-versioned field definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Definition {
    /// Stable id.
    pub id: DefinitionId,
    /// Version (increments on configuration change).
    pub version: i32,
    /// Entity name (`items.item`, …).
    pub entity: String,
    /// Field key within the entity.
    pub key: String,
    /// Stored type.
    pub field_type: FieldType,
    /// Human label.
    pub label: String,
    /// Validation rule string (`gs1-gtin`, `regex:…`, …).
    pub validation_rule: String,
    /// Required on the record.
    pub required: bool,
    /// Indexed hint (per-type btree exists on PK).
    pub indexed: bool,
    /// Owning module id (`mod-udi`, …).
    pub owner_module: String,
    /// Active or retired.
    pub status: DefinitionStatus,
}

/// Input for [`crate::define`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionSpec {
    /// Entity name.
    pub entity: String,
    /// Field key.
    pub key: String,
    /// Stored type.
    pub field_type: FieldType,
    /// Human label.
    pub label: String,
    /// Validation rule.
    pub validation_rule: String,
    /// Required flag.
    pub required: bool,
    /// Indexed flag.
    pub indexed: bool,
    /// Owning module.
    pub owner_module: String,
}

/// Typed value (never a JSON blob).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Value {
    /// Short string.
    String(String),
    /// Long text.
    Text(String),
    /// Integer.
    Integer(i64),
    /// Decimal with scale.
    Decimal {
        /// Numeric value.
        value: Decimal,
        /// Scale.
        scale: i16,
    },
    /// Boolean.
    Bool(bool),
    /// Date with precision.
    Date {
        /// Stored date.
        date: NaiveDate,
        /// Precision discriminator.
        precision: DatePrecision,
    },
    /// Enum member.
    Enum(String),
    /// Reference to another record.
    Reference {
        /// Target entity.
        entity: String,
        /// Target record id.
        id: Identifier,
    },
}

impl Value {
    /// Whether the value is empty for required-field checks.
    pub fn is_empty(&self) -> bool {
        match self {
            Self::String(s) | Self::Text(s) | Self::Enum(s) => s.is_empty(),
            Self::Reference { entity, .. } => entity.is_empty(),
            _ => false,
        }
    }

    /// Expected type for this value.
    pub fn field_type(&self) -> FieldType {
        match self {
            Self::String(_) => FieldType::String,
            Self::Text(_) => FieldType::Text,
            Self::Integer(_) => FieldType::Integer,
            Self::Decimal { .. } => FieldType::Decimal,
            Self::Bool(_) => FieldType::Bool,
            Self::Date { .. } => FieldType::Date,
            Self::Enum(_) => FieldType::Enum,
            Self::Reference { .. } => FieldType::Reference,
        }
    }
}

/// Wire shape for a stored value (`docs/10`).
///
/// [`Self::type_name`] is [`FieldType::as_str`] (`"string"`), not the enum's
/// serde name (`"String"`). [`Self::value`] is the handler `value_payload`
/// JSON, not [`Value`]'s externally tagged serde.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ValueWire {
    /// Field key.
    pub key: String,
    /// Type token from [`FieldType::as_str`].
    #[serde(rename = "type")]
    #[schemars(schema_with = "field_type_token_schema")]
    pub type_name: String,
    /// Payload for [`Self::type_name`], or null when the definition is unset.
    #[schemars(schema_with = "value_payload_schema")]
    pub value: serde_json::Value,
    /// Definition version at write time.
    pub definition_version: i32,
}

fn instance(ty: schemars::schema::InstanceType) -> schemars::schema::Schema {
    schemars::schema::SchemaObject {
        instance_type: Some(ty.into()),
        ..Default::default()
    }
    .into()
}

fn string_format(format: &str) -> schemars::schema::Schema {
    schemars::schema::SchemaObject {
        instance_type: Some(schemars::schema::InstanceType::String.into()),
        format: Some(format.to_owned()),
        ..Default::default()
    }
    .into()
}

fn string_enum(values: Vec<serde_json::Value>) -> schemars::schema::Schema {
    schemars::schema::SchemaObject {
        instance_type: Some(schemars::schema::InstanceType::String.into()),
        enum_values: Some(values),
        ..Default::default()
    }
    .into()
}

fn object_schema(fields: &[(&str, schemars::schema::Schema)]) -> schemars::schema::Schema {
    let mut properties = schemars::Map::new();
    let mut required = schemars::Set::new();
    for (name, schema) in fields {
        required.insert((*name).to_owned());
        properties.insert((*name).to_owned(), schema.clone());
    }
    schemars::schema::SchemaObject {
        instance_type: Some(schemars::schema::InstanceType::Object.into()),
        object: Some(Box::new(schemars::schema::ObjectValidation {
            properties,
            required,
            additional_properties: Some(Box::new(schemars::schema::Schema::from(false))),
            ..Default::default()
        })),
        ..Default::default()
    }
    .into()
}

/// Every [`FieldType`] variant, in declaration order.
fn each_field_type() -> [FieldType; 8] {
    [
        FieldType::String,
        FieldType::Text,
        FieldType::Integer,
        FieldType::Decimal,
        FieldType::Bool,
        FieldType::Date,
        FieldType::Enum,
        FieldType::Reference,
    ]
}

fn field_type_token_values() -> Vec<serde_json::Value> {
    each_field_type()
        .into_iter()
        .map(|ty| {
            let token = match ty {
                FieldType::String => FieldType::String.as_str(),
                FieldType::Text => FieldType::Text.as_str(),
                FieldType::Integer => FieldType::Integer.as_str(),
                FieldType::Decimal => FieldType::Decimal.as_str(),
                FieldType::Bool => FieldType::Bool.as_str(),
                FieldType::Date => FieldType::Date.as_str(),
                FieldType::Enum => FieldType::Enum.as_str(),
                FieldType::Reference => FieldType::Reference.as_str(),
            };
            serde_json::Value::from(token)
        })
        .collect()
}

fn field_type_token_schema(_: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
    string_enum(field_type_token_values())
}

fn date_precision_values() -> Vec<serde_json::Value> {
    [
        DatePrecision::Day,
        DatePrecision::Month,
        DatePrecision::Year,
    ]
    .into_iter()
    .map(|precision| {
        let token = match precision {
            DatePrecision::Day => "day",
            DatePrecision::Month => "month",
            DatePrecision::Year => "year",
        };
        debug_assert_eq!(token, precision.as_str());
        serde_json::Value::from(token)
    })
    .collect()
}

fn value_payload_schema(_: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
    schemars::schema::SchemaObject {
        metadata: Some(Box::new(schemars::schema::Metadata {
            description: Some(
                "null when unset; a string for string/text/enum; an integer; a boolean; \
                 {value, scale} for decimal (value is a decimal string); \
                 {value, precision} for date (value is YYYY-MM-DD, precision is day|month|year); \
                 {entity, id} for reference (id is a uuid)."
                    .into(),
            ),
            ..Default::default()
        })),
        subschemas: Some(Box::new(schemars::schema::SubschemaValidation {
            one_of: Some(vec![
                instance(schemars::schema::InstanceType::Null),
                instance(schemars::schema::InstanceType::String),
                instance(schemars::schema::InstanceType::Integer),
                instance(schemars::schema::InstanceType::Boolean),
                object_schema(&[
                    ("value", instance(schemars::schema::InstanceType::String)),
                    ("scale", instance(schemars::schema::InstanceType::Integer)),
                ]),
                object_schema(&[
                    ("value", string_format("date")),
                    ("precision", string_enum(date_precision_values())),
                ]),
                object_schema(&[
                    ("entity", instance(schemars::schema::InstanceType::String)),
                    ("id", string_format("uuid")),
                ]),
            ]),
            ..Default::default()
        })),
        ..Default::default()
    }
    .into()
}

#[cfg(test)]
mod wire {
    use super::*;
    use serde_json::json;
    use uuid::Uuid;

    fn sample_id() -> DefinitionId {
        DefinitionId::new(Identifier::from_uuid(
            Uuid::parse_str("01234567-89ab-4def-8123-456789abcdef").expect("uuid"),
        ))
    }

    fn sample_def() -> Definition {
        Definition {
            id: sample_id(),
            version: 1,
            entity: "items.item".into(),
            key: "shop_note".into(),
            field_type: FieldType::String,
            label: "Shop note".into(),
            validation_rule: String::new(),
            required: false,
            indexed: false,
            owner_module: "mod-items".into(),
            status: DefinitionStatus::Active,
        }
    }

    #[test]
    fn field_type_serde_is_variant_name_not_as_str() {
        assert_eq!(
            serde_json::to_value(FieldType::String).expect("json"),
            json!("String")
        );
        assert_eq!(FieldType::String.as_str(), "string");
    }

    #[test]
    fn definition_status_serde_is_variant_name_not_as_str() {
        assert_eq!(
            serde_json::to_value(DefinitionStatus::Active).expect("json"),
            json!("Active")
        );
        assert_eq!(
            serde_json::to_value(DefinitionStatus::Retired).expect("json"),
            json!("Retired")
        );
        assert_eq!(DefinitionStatus::Retired.as_str(), "retired");
    }

    #[test]
    fn definition_list_item_shape() {
        let v = serde_json::to_value(sample_def()).expect("json");
        assert_eq!(
            v,
            json!({
                "id": "01234567-89ab-4def-8123-456789abcdef",
                "version": 1,
                "entity": "items.item",
                "key": "shop_note",
                "field_type": "String",
                "label": "Shop note",
                "validation_rule": "",
                "required": false,
                "indexed": false,
                "owner_module": "mod-items",
                "status": "Active",
            })
        );
    }

    #[test]
    fn value_wire_type_is_as_str_token() {
        let w = ValueWire {
            key: "shop_note".into(),
            type_name: FieldType::String.as_str().to_string(),
            value: serde_json::Value::Null,
            definition_version: 1,
        };
        assert_eq!(
            serde_json::to_value(&w).expect("json"),
            json!({
                "key": "shop_note",
                "type": "string",
                "value": null,
                "definition_version": 1,
            })
        );
    }

    #[test]
    fn definition_id_is_a_uuid_string() {
        let v = serde_json::to_value(sample_id()).expect("json");
        assert_eq!(v, json!("01234567-89ab-4def-8123-456789abcdef"));
    }

    fn string_enum_tokens(schema: &serde_json::Value) -> Vec<&str> {
        if let Some(values) = schema.get("enum").and_then(|v| v.as_array()) {
            return values
                .iter()
                .map(|s| s.as_str().expect("enum token"))
                .collect();
        }
        schema["oneOf"]
            .as_array()
            .unwrap_or_else(|| panic!("{schema}"))
            .iter()
            .map(|branch| branch["enum"][0].as_str().expect("enum token"))
            .collect()
    }

    #[test]
    fn field_type_schema_is_variant_names_not_as_str() {
        let v = serde_json::to_value(schemars::schema_for!(FieldType)).expect("schema");
        assert_eq!(
            string_enum_tokens(&v),
            [
                "String",
                "Text",
                "Integer",
                "Decimal",
                "Bool",
                "Date",
                "Enum",
                "Reference"
            ]
        );
    }

    #[test]
    fn definition_status_schema_is_variant_names_not_as_str() {
        let v = serde_json::to_value(schemars::schema_for!(DefinitionStatus)).expect("schema");
        assert_eq!(string_enum_tokens(&v), ["Active", "Retired"]);
    }

    #[test]
    fn value_wire_schema_uses_as_str_tokens_and_payload_shapes() {
        let v = serde_json::to_value(schemars::schema_for!(ValueWire)).expect("schema");
        let type_names: Vec<_> = v["properties"]["type"]["enum"]
            .as_array()
            .expect("type enum")
            .iter()
            .map(|s| s.as_str().expect("type token"))
            .collect();
        assert_eq!(
            type_names,
            each_field_type()
                .iter()
                .map(|ty| ty.as_str())
                .collect::<Vec<_>>()
        );
        let branches = v["properties"]["value"]["oneOf"]
            .as_array()
            .expect("value oneOf");
        assert_eq!(branches.len(), 7, "{branches:?}");
        assert!(v["properties"]["value"].get("enum").is_none());
    }

    #[test]
    fn value_variants_cover_the_payload_schema() {
        fn shape(value: &Value) -> &'static str {
            match value {
                Value::String(_) | Value::Text(_) | Value::Enum(_) => "string",
                Value::Integer(_) => "integer",
                Value::Bool(_) => "boolean",
                Value::Decimal { .. } => "decimal",
                Value::Date { .. } => "date",
                Value::Reference { .. } => "reference",
            }
        }
        assert_eq!(shape(&Value::Integer(1)), "integer");
        assert_eq!(shape(&Value::String("a".into())), "string");
    }
}
