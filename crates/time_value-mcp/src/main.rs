//! `time-value-mcp` — a Model Context Protocol server over the [`time_value`]
//! library.
//!
//! Placeholder entry point. The MCP server exposes `time_value`'s TVM operations
//! as tools over stdio; its design lives in `docs/adr/0011-mcp-server.md`. Async
//! is contained to this crate (ADR-0003) — the core library stays synchronous.

fn main() {
    println!("time-value-mcp: server not yet implemented");
}

#[cfg(test)]
mod tests {
    #[test]
    fn placeholder_builds() {
        assert_eq!(2 + 2, 4);
    }
}
