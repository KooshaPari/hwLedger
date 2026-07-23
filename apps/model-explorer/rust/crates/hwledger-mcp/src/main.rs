//! `hwledger-mcp` — JSON-RPC 2.0 stdio entrypoint for the hwledger MCP server.
//!
//! Reads one JSON-RPC 2.0 message per line from stdin and writes
//! responses to stdout. Run with no arguments; the binary is intended
//! to be launched by an MCP-aware client (e.g. Claude Desktop, an LLM
//! agent harness) as a child process.

use std::io::{self, BufReader};

use hwledger_mcp::{transport, McpServer, McpState};

fn main() {
    let server = McpServer::new();
    let mut state = McpState::new();

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    let reader = BufReader::new(stdin.lock());

    if let Err(e) = transport::run_stdio(&server, &mut state, reader, &mut stdout) {
        // Stdio is the only I/O we have; surface the failure there as a
        // last-resort JSON-RPC error message so the client can observe
        // what went wrong.
        eprintln!("hwledger-mcp: stdio transport failed: {e}");
        std::process::exit(1);
    }
}
