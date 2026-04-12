//! MCP (Model Context Protocol) server — protocol version 2025-03-26.
//!
//! Runs as a stdio JSON-RPC server started by:
//!   `claude mcp add --transport stdio ctxforge -- ctxforge mcp`
//!
//! Reads JSON-RPC requests from stdin (one per line), processes them,
//! writes JSON-RPC responses to stdout. No network, no async.

pub mod prompts;
pub mod protocol;
pub mod resources;
pub mod tools;

use crate::error::Result;
use crate::paths::CtxforgeRoot;
use protocol::{Request, Response};
use serde_json::{Value, json};
use std::io::{self, BufRead, Write};

const SERVER_NAME: &str = "ctxforge";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
const PROTOCOL_VERSION: &str = "2025-03-26";

pub fn run(root: CtxforgeRoot) -> Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let request: Request = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let resp = Response::error(None, -32700, format!("parse error: {e}"));
                write_response(&mut out, &resp)?;
                continue;
            }
        };

        // Notifications (no id) don't get responses.
        let is_notification = request.id.is_none();

        let response = handle_request(&root, &request);

        if !is_notification {
            if let Some(resp) = response {
                write_response(&mut out, &resp)?;
            }
        }
    }

    Ok(())
}

fn handle_request(root: &CtxforgeRoot, req: &Request) -> Option<Response> {
    match req.method.as_str() {
        "initialize" => Some(Response::success(
            req.id.clone(),
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {
                    "tools": {},
                    "resources": {},
                    "prompts": {}
                },
                "serverInfo": {
                    "name": SERVER_NAME,
                    "version": SERVER_VERSION
                }
            }),
        )),

        // Notification — no response.
        "notifications/initialized" => None,

        "tools/list" => Some(Response::success(req.id.clone(), tools::tool_list())),

        "tools/call" => {
            let tool_name = req
                .params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let arguments = req
                .params
                .get("arguments")
                .cloned()
                .unwrap_or(Value::Object(Default::default()));

            match tools::call_tool(root, tool_name, &arguments) {
                Ok(content) => Some(Response::success(
                    req.id.clone(),
                    json!({ "content": [content] }),
                )),
                Err(e) => Some(Response::error(req.id.clone(), -32000, e)),
            }
        }

        "resources/list" => {
            Some(Response::success(req.id.clone(), resources::resource_list()))
        }

        "resources/read" => {
            let uri = req
                .params
                .get("uri")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            match resources::read_resource(root, uri) {
                Ok(content) => Some(Response::success(req.id.clone(), content)),
                Err(e) => Some(Response::error(req.id.clone(), -32000, e)),
            }
        }

        "prompts/list" => Some(Response::success(req.id.clone(), prompts::prompt_list())),

        "prompts/get" => {
            let name = req
                .params
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let arguments = req
                .params
                .get("arguments")
                .cloned()
                .unwrap_or(Value::Object(Default::default()));
            match prompts::get_prompt(root, name, &arguments) {
                Ok(content) => Some(Response::success(req.id.clone(), content)),
                Err(e) => Some(Response::error(req.id.clone(), -32000, e)),
            }
        }

        // Unknown method.
        _ => Some(Response::error(
            req.id.clone(),
            -32601,
            format!("method not found: {}", req.method),
        )),
    }
}

fn write_response(out: &mut impl Write, resp: &Response) -> io::Result<()> {
    let json = serde_json::to_string(resp)?;
    writeln!(out, "{json}")?;
    out.flush()
}
