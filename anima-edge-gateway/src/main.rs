//! Edge Translation Gateway — INP ↔ MCP/A2A JSON bridge for TPT Anima.
//!
//! Reads a JSON INP intent from stdin, translates to MCP or A2A, and prints
//! the result to stdout. Usage:
//!
//!     echo '{"id":"1","action":"do","payload":{}}' | anima-edge-gateway --mode mcp
//!     echo '{"id":"1","action":"do","payload":{}}' | anima-edge-gateway --mode a2a

use anima_edge_gateway::{InpIntent, InpToA2aTranslator, InpToMcpTranslator};
use std::io::{self, Read};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 || args[1] != "--mode" || args.len() < 3 {
        eprintln!("Usage: anima-edge-gateway --mode <mcp|a2a>");
        eprintln!();
        eprintln!("Reads an INP intent JSON from stdin and prints the translated output.");
        eprintln!();
        eprintln!("Examples:");
        eprintln!("  echo '{{\"id\":\"1\",\"action\":\"do\",\"payload\":{{}}}}' | anima-edge-gateway --mode mcp");
        eprintln!("  echo '{{\"id\":\"1\",\"action\":\"do\",\"payload\":{{\"message\":\"hi\"}}}}' | anima-edge-gateway --mode a2a");
        std::process::exit(1);
    }

    let mode = args[2].to_lowercase();
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .expect("Failed to read stdin");

    let intent: InpIntent = match serde_json::from_str(&input) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error: invalid INP intent JSON: {e}");
            std::process::exit(1);
        }
    };

    match mode.as_str() {
        "mcp" => {
            let mcp = InpToMcpTranslator::translate(&intent);
            println!("{}", serde_json::to_string_pretty(&mcp).unwrap());
        }
        "a2a" => {
            let a2a = InpToA2aTranslator::translate(&intent);
            println!("{}", serde_json::to_string_pretty(&a2a).unwrap());
        }
        other => {
            eprintln!("Error: unknown mode '{other}'. Expected 'mcp' or 'a2a'.");
            std::process::exit(1);
        }
    }
}
