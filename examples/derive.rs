//! Derive a JSON Schema for a type, and show it next to the serde wire form of
//! each value.
//!
//! Run with: `cargo run --example derive`

use enum2schema::Schema;
use serde::Serialize;

/// A drawing command.
#[derive(Schema, Serialize)]
enum Command {
    /// Clear the canvas.
    Clear,
    /// Draw a circle.
    Circle {
        /// Center as [x, y].
        center: [f32; 2],
        radius: f32,
    },
    /// Set the active color name.
    SetColor(String),
}

fn main() {
    println!("schema:");
    println!(
        "{}",
        serde_json::to_string_pretty(&Command::schema()).unwrap()
    );

    let commands = [
        Command::Clear,
        Command::Circle {
            center: [1.0, 2.0],
            radius: 3.0,
        },
        Command::SetColor("red".to_string()),
    ];
    println!("\nvalues:");
    for command in &commands {
        println!("{}", serde_json::to_string(command).unwrap());
    }
}
