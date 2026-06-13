use enum2schema::Schema;
use serde::Serialize;
use serde_json::json;

#[derive(Schema, Serialize)]
enum Color {
    Red,
    Green,
    Blue,
}

#[test]
fn unit_enum_yields_oneof_of_string_consts() {
    assert_eq!(
        Color::schema(),
        json!({
            "oneOf": [
                { "type": "string", "const": "Red" },
                { "type": "string", "const": "Green" },
                { "type": "string", "const": "Blue" }
            ]
        })
    );
}

#[test]
fn unit_enum_wire_format_matches_schema_consts() {
    for (variant, expected) in [
        (Color::Red, "Red"),
        (Color::Green, "Green"),
        (Color::Blue, "Blue"),
    ] {
        assert_eq!(serde_json::to_value(&variant).unwrap(), json!(expected));
    }
}

/// A command for the worker.
#[derive(Schema, Serialize)]
enum Command {
    /// Stop everything.
    Stop,
    /// Move to a point.
    Move {
        x: f32,
        y: f32,
    },
    Tag(String),
    Pair(u8, u8),
}

#[test]
fn mixed_enum_yields_externally_tagged_forms() {
    assert_eq!(
        Command::schema(),
        json!({
            "description": "A command for the worker.",
            "oneOf": [
                { "type": "string", "const": "Stop", "description": "Stop everything." },
                {
                    "type": "object",
                    "properties": {
                        "Move": {
                            "type": "object",
                            "properties": {
                                "x": { "type": "number" },
                                "y": { "type": "number" }
                            },
                            "required": ["x", "y"]
                        }
                    },
                    "required": ["Move"],
                    "description": "Move to a point."
                },
                {
                    "type": "object",
                    "properties": { "Tag": { "type": "string" } },
                    "required": ["Tag"]
                },
                {
                    "type": "object",
                    "properties": {
                        "Pair": {
                            "type": "array",
                            "prefixItems": [{ "type": "integer" }, { "type": "integer" }],
                            "minItems": 2,
                            "maxItems": 2
                        }
                    },
                    "required": ["Pair"]
                }
            ]
        })
    );
}

#[test]
fn mixed_enum_wire_format_is_externally_tagged() {
    assert_eq!(serde_json::to_value(Command::Stop).unwrap(), json!("Stop"));
    assert_eq!(
        serde_json::to_value(Command::Move { x: 1.0, y: 2.0 }).unwrap(),
        json!({ "Move": { "x": 1.0, "y": 2.0 } })
    );
    assert_eq!(
        serde_json::to_value(Command::Tag("hi".to_string())).unwrap(),
        json!({ "Tag": "hi" })
    );
    assert_eq!(
        serde_json::to_value(Command::Pair(3, 4)).unwrap(),
        json!({ "Pair": [3, 4] })
    );
}

/// A configuration block.
#[derive(Schema, Serialize)]
struct Config {
    /// The display name.
    name: String,
    tags: Vec<String>,
    matrix: [u8; 4],
    timeout: Option<u64>,
}

#[test]
fn struct_descriptions_arrays_and_optional_fields() {
    assert_eq!(
        Config::schema(),
        json!({
            "type": "object",
            "description": "A configuration block.",
            "properties": {
                "name": { "type": "string", "description": "The display name." },
                "tags": { "type": "array", "items": { "type": "string" } },
                "matrix": { "type": "array", "items": { "type": "integer" }, "minItems": 4, "maxItems": 4 },
                "timeout": { "type": ["integer", "null"] }
            },
            "required": ["name", "tags", "matrix"]
        })
    );
}

#[test]
fn struct_wire_format_round_trips() {
    let config = Config {
        name: "demo".to_string(),
        tags: vec!["a".to_string()],
        matrix: [1, 2, 3, 4],
        timeout: None,
    };
    assert_eq!(
        serde_json::to_value(&config).unwrap(),
        json!({
            "name": "demo",
            "tags": ["a"],
            "matrix": [1, 2, 3, 4],
            "timeout": null
        })
    );
}

#[derive(Serialize)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

fn rgba_schema() -> serde_json::Value {
    json!({
        "type": "array",
        "items": { "type": "number" },
        "minItems": 4,
        "maxItems": 4
    })
}

#[derive(Schema, Serialize)]
struct Material {
    #[schema(with = rgba_schema)]
    color: [f32; 4],
    #[schema(type = "array", items = "number", len = 3)]
    position: Vec3,
}

#[test]
fn with_and_type_attributes_override_field_schemas() {
    assert_eq!(
        Material::schema(),
        json!({
            "type": "object",
            "properties": {
                "color": {
                    "type": "array",
                    "items": { "type": "number" },
                    "minItems": 4,
                    "maxItems": 4
                },
                "position": {
                    "type": "array",
                    "items": { "type": "number" },
                    "minItems": 3,
                    "maxItems": 3
                }
            },
            "required": ["color", "position"]
        })
    );
}

#[test]
fn override_fields_still_serialize() {
    let material = Material {
        color: [0.5, 0.25, 0.75, 1.0],
        position: Vec3 {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
    };
    assert_eq!(
        serde_json::to_value(&material).unwrap(),
        json!({
            "color": [0.5, 0.25, 0.75, 1.0],
            "position": { "x": 1.0, "y": 2.0, "z": 3.0 }
        })
    );
}

#[derive(Schema, Serialize)]
#[serde(rename_all = "camelCase")]
struct Settings {
    max_count: u32,
    #[serde(default)]
    retry_limit: u32,
    #[serde(rename = "id")]
    identifier: String,
    #[schema(skip)]
    internal: bool,
    #[schema(description = "A free-form note.")]
    note: String,
}

#[test]
fn serde_rename_default_and_schema_skip() {
    assert_eq!(
        Settings::schema(),
        json!({
            "type": "object",
            "properties": {
                "maxCount": { "type": "integer" },
                "retryLimit": { "type": "integer" },
                "id": { "type": "string" },
                "note": { "type": "string", "description": "A free-form note." }
            },
            "required": ["maxCount", "id", "note"]
        })
    );
}

#[test]
fn settings_property_names_match_serde_wire_names() {
    let settings = Settings {
        max_count: 5,
        retry_limit: 2,
        identifier: "abc".to_string(),
        internal: true,
        note: "hello".to_string(),
    };
    assert_eq!(
        serde_json::to_value(&settings).unwrap(),
        json!({
            "maxCount": 5,
            "retryLimit": 2,
            "id": "abc",
            "internal": true,
            "note": "hello"
        })
    );
}

#[derive(Schema, Serialize)]
#[serde(rename_all = "snake_case")]
enum Event {
    UserJoined,
    UserLeft,
}

#[test]
fn serde_rename_all_applies_to_variants() {
    assert_eq!(
        Event::schema(),
        json!({
            "oneOf": [
                { "type": "string", "const": "user_joined" },
                { "type": "string", "const": "user_left" }
            ]
        })
    );
}

#[test]
fn event_rename_all_wire_format_matches_schema() {
    assert_eq!(
        serde_json::to_value(Event::UserJoined).unwrap(),
        json!("user_joined")
    );
    assert_eq!(
        serde_json::to_value(Event::UserLeft).unwrap(),
        json!("user_left")
    );
}

#[derive(Schema, Serialize)]
#[schema(string_enum)]
enum PrimitiveKind {
    Cube,
    Sphere,
    Cylinder,
}

#[test]
fn string_enum_yields_single_type_with_enum_list() {
    assert_eq!(
        PrimitiveKind::schema(),
        json!({ "type": "string", "enum": ["Cube", "Sphere", "Cylinder"] })
    );
}

#[test]
fn string_enum_wire_format_matches_enum_list() {
    for (variant, expected) in [
        (PrimitiveKind::Cube, "Cube"),
        (PrimitiveKind::Sphere, "Sphere"),
        (PrimitiveKind::Cylinder, "Cylinder"),
    ] {
        assert_eq!(serde_json::to_value(&variant).unwrap(), json!(expected));
    }
}

#[test]
fn empty_struct_omits_required() {
    #[derive(Schema)]
    struct NoArgs {}

    assert_eq!(
        NoArgs::schema(),
        json!({ "type": "object", "properties": {} })
    );
}

#[test]
fn value_tuple_and_map_leaf_schemas() {
    assert_eq!(<serde_json::Value as Schema>::schema(), json!({}));
    assert_eq!(<() as Schema>::schema(), json!({ "type": "null" }));
    assert_eq!(
        <(String, u32) as Schema>::schema(),
        json!({
            "type": "array",
            "prefixItems": [{ "type": "string" }, { "type": "integer" }],
            "minItems": 2,
            "maxItems": 2
        })
    );
    assert_eq!(
        <std::collections::HashMap<String, f32> as Schema>::schema(),
        json!({ "type": "object", "additionalProperties": { "type": "number" } })
    );
    assert_eq!(
        <serde_json::Map<String, serde_json::Value> as Schema>::schema(),
        json!({ "type": "object", "additionalProperties": true })
    );
}

#[test]
fn smart_pointers_are_transparent() {
    assert_eq!(
        <Box<String> as Schema>::schema(),
        json!({ "type": "string" })
    );
    assert_eq!(
        <std::rc::Rc<u32> as Schema>::schema(),
        json!({ "type": "integer" })
    );
    assert_eq!(
        <std::sync::Arc<bool> as Schema>::schema(),
        json!({ "type": "boolean" })
    );
    assert_eq!(<Box<Command> as Schema>::schema(), Command::schema());
}

#[test]
fn fixed_arrays_carry_their_length() {
    assert_eq!(
        <[f32; 4] as Schema>::schema(),
        json!({ "type": "array", "items": { "type": "number" }, "minItems": 4, "maxItems": 4 })
    );
}

/// A generic envelope.
#[derive(Schema, Serialize)]
struct Envelope<T> {
    id: u32,
    result: Option<T>,
}

#[test]
fn generic_struct_uses_inner_schema() {
    assert_eq!(
        Envelope::<String>::schema(),
        json!({
            "type": "object",
            "description": "A generic envelope.",
            "properties": {
                "id": { "type": "integer" },
                "result": { "type": ["string", "null"] }
            },
            "required": ["id"]
        })
    );
}

#[test]
fn generic_struct_wire_format_round_trips() {
    let envelope = Envelope {
        id: 1,
        result: Some("ok".to_string()),
    };
    assert_eq!(
        serde_json::to_value(&envelope).unwrap(),
        json!({ "id": 1, "result": "ok" })
    );
}

#[test]
fn mcp_tool_descriptor_embeds_input_schema() {
    assert_eq!(
        enum2schema::mcp::tool::<Config>("save_config", "Save the config"),
        json!({
            "name": "save_config",
            "description": "Save the config",
            "inputSchema": Config::schema()
        })
    );
}

#[test]
fn mcp_tool_io_descriptor_embeds_both_schemas() {
    assert_eq!(
        enum2schema::mcp::tool_io::<Config, Color>("do", "Do it"),
        json!({
            "name": "do",
            "description": "Do it",
            "inputSchema": Config::schema(),
            "outputSchema": Color::schema()
        })
    );
}

/// An inner record.
#[derive(Schema, Serialize)]
struct Inner {
    value: u32,
}

/// An outer record that nests a derived type directly and in a list.
#[derive(Schema, Serialize)]
struct Outer {
    inner: Inner,
    list: Vec<Inner>,
}

#[test]
fn nested_derived_types_recurse() {
    let inner_schema = json!({
        "type": "object",
        "description": "An inner record.",
        "properties": { "value": { "type": "integer" } },
        "required": ["value"]
    });
    assert_eq!(
        Outer::schema(),
        json!({
            "type": "object",
            "description": "An outer record that nests a derived type directly and in a list.",
            "properties": {
                "inner": inner_schema,
                "list": { "type": "array", "items": inner_schema }
            },
            "required": ["inner", "list"]
        })
    );
}

#[test]
fn nested_types_wire_round_trip() {
    let outer = Outer {
        inner: Inner { value: 1 },
        list: vec![Inner { value: 2 }, Inner { value: 3 }],
    };
    assert_eq!(
        serde_json::to_value(&outer).unwrap(),
        json!({ "inner": { "value": 1 }, "list": [{ "value": 2 }, { "value": 3 }] })
    );
}

/// A message; the internal variant is not advertised in the schema.
#[derive(Schema, Serialize)]
enum Message {
    Ping,
    Echo {
        text: String,
    },
    #[schema(skip)]
    Internal(u32),
}

#[test]
fn skip_omits_data_enum_variant() {
    assert_eq!(
        Message::schema(),
        json!({
            "description": "A message; the internal variant is not advertised in the schema.",
            "oneOf": [
                { "type": "string", "const": "Ping" },
                {
                    "type": "object",
                    "properties": {
                        "Echo": {
                            "type": "object",
                            "properties": { "text": { "type": "string" } },
                            "required": ["text"]
                        }
                    },
                    "required": ["Echo"]
                }
            ]
        })
    );
}

#[test]
fn skipped_variant_still_serializes() {
    assert_eq!(serde_json::to_value(Message::Ping).unwrap(), json!("Ping"));
    assert_eq!(
        serde_json::to_value(Message::Echo {
            text: "hi".to_string()
        })
        .unwrap(),
        json!({ "Echo": { "text": "hi" } })
    );
    assert_eq!(
        serde_json::to_value(Message::Internal(5)).unwrap(),
        json!({ "Internal": 5 })
    );
}

#[derive(Schema, Serialize)]
#[schema(string_enum)]
enum Mode {
    On,
    Off,
    #[schema(skip)]
    Debug,
}

#[test]
fn skip_omits_string_enum_variant() {
    assert_eq!(
        Mode::schema(),
        json!({ "type": "string", "enum": ["On", "Off"] })
    );
}

#[test]
fn skipped_string_enum_variant_still_serializes() {
    assert_eq!(serde_json::to_value(Mode::On).unwrap(), json!("On"));
    assert_eq!(serde_json::to_value(Mode::Off).unwrap(), json!("Off"));
    assert_eq!(serde_json::to_value(Mode::Debug).unwrap(), json!("Debug"));
}

#[test]
fn btreemap_is_object_with_additional_properties() {
    assert_eq!(
        <std::collections::BTreeMap<String, u32> as Schema>::schema(),
        json!({ "type": "object", "additionalProperties": { "type": "integer" } })
    );
}

#[test]
fn sets_are_unique_arrays() {
    assert_eq!(
        <std::collections::BTreeSet<String> as Schema>::schema(),
        json!({ "type": "array", "items": { "type": "string" }, "uniqueItems": true })
    );
}

#[derive(Schema, Serialize)]
struct Bag {
    #[schema(description = "Map of component name to value.")]
    components: serde_json::Map<String, serde_json::Value>,
    pairs: Vec<(String, serde_json::Value)>,
}

#[test]
fn component_bag_and_pair_list_schema_and_wire() {
    assert_eq!(
        Bag::schema(),
        json!({
            "type": "object",
            "properties": {
                "components": {
                    "type": "object",
                    "additionalProperties": true,
                    "description": "Map of component name to value."
                },
                "pairs": {
                    "type": "array",
                    "items": {
                        "type": "array",
                        "prefixItems": [{ "type": "string" }, {}],
                        "minItems": 2,
                        "maxItems": 2
                    }
                }
            },
            "required": ["components", "pairs"]
        })
    );

    let mut components = serde_json::Map::new();
    components.insert("name".to_string(), json!("cube"));
    let bag = Bag {
        components,
        pairs: vec![("local_transform".to_string(), json!({ "scale": 2 }))],
    };
    assert_eq!(
        serde_json::to_value(&bag).unwrap(),
        json!({
            "components": { "name": "cube" },
            "pairs": [["local_transform", { "scale": 2 }]]
        })
    );
}

fn token_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": { "id": { "type": "integer" } },
        "required": ["id"]
    })
}

#[derive(Schema, Serialize)]
enum Reference {
    Token(#[schema(with = token_schema)] u64),
    Index(u32),
}

#[test]
fn with_override_applies_to_tuple_variant_field() {
    assert_eq!(
        Reference::schema(),
        json!({
            "oneOf": [
                { "type": "object", "properties": { "Token": token_schema() }, "required": ["Token"] },
                { "type": "object", "properties": { "Index": { "type": "integer" } }, "required": ["Index"] }
            ]
        })
    );
}

#[test]
fn with_override_tuple_variant_wire_format_matches_tags() {
    assert_eq!(
        serde_json::to_value(Reference::Token(7)).unwrap(),
        json!({ "Token": 7 })
    );
    assert_eq!(
        serde_json::to_value(Reference::Index(3)).unwrap(),
        json!({ "Index": 3 })
    );
}
