//! Describe an MCP tool straight from its Rust types.
//!
//! Run with: `cargo run --example mcp_tool`
//!
//! enum2schema supplies the schemas (inputSchema/outputSchema); serde supplies
//! the actual request/response values. Neither needs a hand written literal.

use enum2schema::Schema;
use serde::{Deserialize, Serialize};

/// Place an entity in the scene.
#[derive(Schema, Serialize, Deserialize)]
struct PlaceArgs {
    /// The entity to place.
    entity: u32,
    /// World position as [x, y, z].
    position: [f32; 3],
    /// Optional material name to assign.
    material: Option<String>,
}

/// The result of placing an entity.
#[derive(Schema, Serialize, Deserialize)]
struct PlaceReply {
    placed: bool,
    version: u64,
}

fn main() {
    // Schema side: the MCP tool descriptor is derived from the types.
    let descriptor =
        enum2schema::mcp::tool_io::<PlaceArgs, PlaceReply>("place", "Place an entity in the scene");
    println!("tool descriptor:");
    println!("{}", serde_json::to_string_pretty(&descriptor).unwrap());

    // Value side: serde builds the request and response payloads.
    let request = PlaceArgs {
        entity: 7,
        position: [1.0, 0.0, 2.0],
        material: Some("wood".to_string()),
    };
    println!("\nrequest:  {}", serde_json::to_string(&request).unwrap());

    let response = PlaceReply {
        placed: true,
        version: 42,
    };
    println!("response: {}", serde_json::to_string(&response).unwrap());
}
