//! `time-value-mcp` — a Model Context Protocol server over the [`time_value`]
//! library.
//!
//! Speaks MCP over **stdio** via the official `rmcp` SDK, exposing the library's
//! time-value-of-money operations as tools (see `docs/adr/0011-mcp-server.md`).
//! Async lives only here (ADR-0003); the tools call the synchronous, `no_std`
//! core directly.

mod params;
mod server;

use anyhow::Result;
use rmcp::{transport::stdio, ServiceExt};

use crate::server::TimeValueServer;

#[tokio::main]
async fn main() -> Result<()> {
    let service = TimeValueServer::new().serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
