//! Exposes the turn-end command batch to a harness over local MCP stdio.

use crate::{config, run};
use rmcp::model::{
    CallToolRequestParam, CallToolResult, Content, ErrorData, Implementation, ListToolsResult,
    PaginatedRequestParam, ServerCapabilities, ServerInfo, Tool,
};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::{ServerHandler, ServiceExt};
use serde_json::json;
use std::path::Path;
use std::process::ExitCode;

const TOOL: &str = "run_turn_end";

struct Latch;

impl ServerHandler for Latch {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            ..Default::default()
        }
    }

    async fn list_tools(
        &self,
        _: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let schema = json!({"type":"object","properties":{},"additionalProperties":false});
        Ok(ListToolsResult {
            tools: vec![Tool::new(
                TOOL,
                "Run the project's configured Latch turn-end commands now and return their results. Commands may modify the workspace.",
                schema
                    .as_object()
                    .expect("tool schema is an object")
                    .clone(),
            )],
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        if request.name != TOOL || request.arguments.is_some_and(|args| !args.is_empty()) {
            return Err(ErrorData::invalid_params(
                "expected run_turn_end with no arguments",
                None,
            ));
        }
        let directory = Path::new(".");
        let result = match config::load(directory, config::HookName::TurnEnd) {
            Ok(Some(entries)) => {
                let report = run::run(directory, entries.into_vec());
                let message = report.message();
                if report.failed() {
                    CallToolResult::error(vec![Content::text(message)])
                } else if message.is_empty() {
                    CallToolResult::success(vec![])
                } else {
                    CallToolResult::success(vec![Content::text(message)])
                }
            }
            Ok(None) => {
                CallToolResult::error(vec![Content::text("hooks.turn_end is not configured")])
            }
            Err(error) => CallToolResult::error(vec![Content::text(error.to_string())]),
        };
        Ok(result)
    }
}

pub(crate) fn dispatch() -> ExitCode {
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        tokio::runtime::Builder::new_current_thread()
            .build()?
            .block_on(async {
                Latch
                    .serve(rmcp::transport::stdio())
                    .await?
                    .waiting()
                    .await?;
                Ok(())
            })
    })();
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("latch: MCP transport failed: {error}");
            ExitCode::FAILURE
        }
    }
}
