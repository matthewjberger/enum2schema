pub use enum2schema_derive::Schema;
pub use serde_json;

use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

pub trait Schema {
    fn schema() -> Value;
}

pub fn set_description(mut schema: Value, description: &str) -> Value {
    if let Value::Object(ref mut map) = schema {
        map.insert(
            "description".to_string(),
            Value::String(description.to_string()),
        );
    }
    schema
}

pub fn nullable(schema: Value) -> Value {
    if let Value::Object(ref map) = schema
        && let Some(Value::String(type_name)) = map.get("type")
    {
        let mut nullable_map = map.clone();
        nullable_map.insert(
            "type".to_string(),
            Value::Array(vec![
                Value::String(type_name.clone()),
                Value::String("null".to_string()),
            ]),
        );
        return Value::Object(nullable_map);
    }
    json!({ "anyOf": [schema, { "type": "null" }] })
}

macro_rules! impl_scalar_schema {
    ($type_name:expr; $($rust_type:ty),+ $(,)?) => {
        $(
            impl Schema for $rust_type {
                fn schema() -> Value {
                    json!({ "type": $type_name })
                }
            }
        )+
    };
}

impl_scalar_schema!("number"; f32, f64);
impl_scalar_schema!("integer"; i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);
impl_scalar_schema!("boolean"; bool);
impl_scalar_schema!("string"; String, str);

impl<T: Schema + ?Sized> Schema for &T {
    fn schema() -> Value {
        T::schema()
    }
}

impl<T: Schema + ?Sized> Schema for Box<T> {
    fn schema() -> Value {
        T::schema()
    }
}

impl<T: Schema + ?Sized> Schema for std::rc::Rc<T> {
    fn schema() -> Value {
        T::schema()
    }
}

impl<T: Schema + ?Sized> Schema for std::sync::Arc<T> {
    fn schema() -> Value {
        T::schema()
    }
}

impl<T: Schema> Schema for HashSet<T> {
    fn schema() -> Value {
        json!({ "type": "array", "items": T::schema(), "uniqueItems": true })
    }
}

impl<T: Schema> Schema for BTreeSet<T> {
    fn schema() -> Value {
        json!({ "type": "array", "items": T::schema(), "uniqueItems": true })
    }
}

impl<T: Schema> Schema for Vec<T> {
    fn schema() -> Value {
        json!({ "type": "array", "items": T::schema() })
    }
}

impl<T: Schema> Schema for [T] {
    fn schema() -> Value {
        json!({ "type": "array", "items": T::schema() })
    }
}

impl<T: Schema, const N: usize> Schema for [T; N] {
    fn schema() -> Value {
        json!({ "type": "array", "items": T::schema(), "minItems": N, "maxItems": N })
    }
}

impl<T: Schema> Schema for Option<T> {
    fn schema() -> Value {
        nullable(T::schema())
    }
}

impl Schema for () {
    fn schema() -> Value {
        json!({ "type": "null" })
    }
}

impl Schema for Value {
    fn schema() -> Value {
        json!({})
    }
}

impl Schema for Map<String, Value> {
    fn schema() -> Value {
        json!({ "type": "object", "additionalProperties": true })
    }
}

impl<V: Schema> Schema for HashMap<String, V> {
    fn schema() -> Value {
        json!({ "type": "object", "additionalProperties": V::schema() })
    }
}

impl<V: Schema> Schema for BTreeMap<String, V> {
    fn schema() -> Value {
        json!({ "type": "object", "additionalProperties": V::schema() })
    }
}

macro_rules! impl_tuple_schema {
    ($count:literal; $($name:ident),+) => {
        impl<$($name: Schema),+> Schema for ($($name,)+) {
            fn schema() -> Value {
                json!({
                    "type": "array",
                    "items": [ $( $name::schema() ),+ ],
                    "minItems": $count,
                    "maxItems": $count
                })
            }
        }
    };
}

impl_tuple_schema!(1; A);
impl_tuple_schema!(2; A, B);
impl_tuple_schema!(3; A, B, C);
impl_tuple_schema!(4; A, B, C, D);
impl_tuple_schema!(5; A, B, C, D, E);
impl_tuple_schema!(6; A, B, C, D, E, F);
impl_tuple_schema!(7; A, B, C, D, E, F, G);
impl_tuple_schema!(8; A, B, C, D, E, F, G, H);
impl_tuple_schema!(9; A, B, C, D, E, F, G, H, I);
impl_tuple_schema!(10; A, B, C, D, E, F, G, H, I, J);
impl_tuple_schema!(11; A, B, C, D, E, F, G, H, I, J, K);
impl_tuple_schema!(12; A, B, C, D, E, F, G, H, I, J, K, L);

/// Helpers for describing MCP tools directly from Rust types, so a tool's
/// `inputSchema` (and optional `outputSchema`) lives next to its argument and
/// result types instead of in a hand written literal.
pub mod mcp {
    use crate::Schema;
    use serde_json::{Value, json};

    /// An MCP tool descriptor `{ name, description, inputSchema }`, with the
    /// input schema derived from the argument type `A`.
    pub fn tool<A: Schema>(name: &str, description: &str) -> Value {
        json!({
            "name": name,
            "description": description,
            "inputSchema": A::schema(),
        })
    }

    /// An MCP tool descriptor that also advertises an `outputSchema` derived from
    /// the result type `O`, for structured tool results.
    pub fn tool_io<A: Schema, O: Schema>(name: &str, description: &str) -> Value {
        json!({
            "name": name,
            "description": description,
            "inputSchema": A::schema(),
            "outputSchema": O::schema(),
        })
    }
}
