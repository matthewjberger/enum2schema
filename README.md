# enum2schema

[<img alt="github" src="https://img.shields.io/badge/github-matthewjberger/enum2schema-8da0cb?style=for-the-badge&labelColor=555555&logo=github" height="20">](https://github.com/matthewjberger/enum2schema)
[<img alt="crates.io" src="https://img.shields.io/crates/v/enum2schema.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/enum2schema)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-enum2schema-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" height="20">](https://docs.rs/enum2schema)

`enum2schema` derives a JSON Schema from a Rust type, so the schema a consumer reads (for example an MCP tool `inputSchema`) lives next to the type instead of in a separate hand written `serde_json::json!` literal.

It carries `///` doc comments into `description`, models enums with serde's default externally tagged representation, and maps Rust types to their JSON Schema equivalents. Leaf types that should not derive `Schema`, such as math types or fixed color arrays, are described with field attributes.

## Why enum2schema

enum2schema is small and focused: a derive that emits an inline JSON Schema matching serde's output, for when a schema exists so a person or an agent can read it (for example an MCP tool `inputSchema`).

- **Tiny dependency.** Just `serde_json` and a derive, with no schema runtime to pull into a game engine or a wasm worker that only needs to describe a handful of types.
- **Inline, not `$ref`.** Every schema is self-contained, which is easier for an LLM (or a human) to read than a web of `$ref`s into a `definitions` table. Recursive types drop the recursive edge with `#[schema(skip)]`.
- **Built for non-deriving leaf types.** Math vectors, quaternions, matrices, and fixed color arrays are described in place with `#[schema(type = "array", items = "number", len = 3)]` or `#[schema(with = ...)]`, with no newtype wrappers or upstream impls for types like `nalgebra`'s.
- **Matched to serde.** It models serde's default externally tagged enums (plus a `string_enum` mode) and respects `rename`, `rename_all`, and `default`, so the schema reflects the real wire format.

It targets serde's externally tagged enums and inline schemas, which is what most tool and message contracts use.

## Usage

Add this to your `Cargo.toml`:

```toml
enum2schema = "0.1"
```

Example:

```rust
use enum2schema::Schema;

/// A command for the worker.
#[derive(Schema)]
enum Command {
    /// Stop everything.
    Stop,
    /// Move to a point.
    Move { x: f32, y: f32 },
}

let schema = Command::schema();
```

`schema()` returns a `serde_json::Value` holding a JSON Schema fragment. The consumer decides how to wrap it: a host wraps an args struct's schema as an MCP tool `inputSchema`; a worker uses a type's schema directly.

## Types

Scalars map to `number`, `integer`, `boolean`, and `string`. `Option<T>` is left out of `required` and made nullable. `Vec<T>`, slices, and sets become `array`; fixed length arrays `[T; N]` and tuples become an `array` carrying `minItems`/`maxItems`. `HashMap`/`BTreeMap` with string keys become an `object` with `additionalProperties`. `Box`, `Rc`, and `Arc` are transparent. `serde_json::Value` becomes the any schema. Nested types that derive `Schema` recurse. Generic types are supported: each type parameter is bounded by `Schema`, so `Envelope<T>` derives a schema once `T: Schema`.

Enums use serde's externally tagged form: a unit variant is a string const inside a `oneOf`, and a data variant is a single key object. For a closed set of unit variants, `#[schema(string_enum)]` emits a single `string` type with an `enum` list instead.

## MCP tools

The `mcp` module builds an MCP tool descriptor straight from the argument type, so the `inputSchema` lives next to the args struct instead of a hand written literal:

```rust
use enum2schema::Schema;

/// Move an entity.
#[derive(Schema)]
struct MoveArgs {
    entity: u32,
    to: [f32; 3],
}

let descriptor = enum2schema::mcp::tool::<MoveArgs>("move", "Move an entity");
// { "name": "move", "description": "Move an entity", "inputSchema": <MoveArgs schema> }
```

`tool_io::<Args, Reply>(name, description)` additionally emits an `outputSchema` from the result type, for MCP structured tool results. The actual request and response *values* are still built by `serde` (`serde_json::to_value`); enum2schema only supplies the schemas.

## Attributes

Field attributes:

- `#[schema(with = path::to::fn)]` uses `path::to::fn()` (returning `serde_json::Value`) as the field schema.
- `#[schema(type = "array", items = "number", len = 3)]` describes a fixed shape for a type that does not implement `Schema`.
- `#[schema(description = "...")]` sets or overrides the description.
- `#[schema(skip)]` omits the field, or, on an enum variant, omits that variant from the schema (serde still serializes it). Use it to drop a recursive variant so an otherwise cyclic type can derive `Schema`.

Container attribute:

- `#[schema(string_enum)]` on a unit only enum emits a `string` type with an `enum` list.

The serde attributes `#[serde(rename = "...")]`, `#[serde(rename_all = "...")]`, and `#[serde(default)]` (treated as not required) are respected.

A single `use enum2schema::Schema;` brings in both the trait and the derive; the crate also re-exports `serde_json`.
