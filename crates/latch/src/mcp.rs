//! Exposes turn-end checks and saved output search over local MCP stdio.

use crate::{config, output, run, status};
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ClientJsonRpcMessage,
    ClientNotification, ContentBlock, ErrorData, JsonRpcMessage, ListToolsResult,
    PaginatedRequestParams, ServerCapabilities, ServerConfig, ServerJsonRpcMessage, Tool,
    ToolAnnotations,
};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::transport::{Transport, async_rw::AsyncRwTransport};
use rmcp::{ServerHandler, ServiceExt};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::path::PathBuf;
use std::process::ExitCode;

const STATUS_TOOL: &str = "latch_status";
const RUN_TOOL: &str = "run_turn_end";
const SEARCH_TOOL: &str = "search_output";

pub(crate) enum ProjectDirectory {
    WorkingDirectory,
    ToolArgument,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolArguments {
    project_directory: PathBuf,
}

impl ProjectDirectory {
    fn resolve(&self, arguments: Option<Map<String, Value>>) -> Result<PathBuf, ErrorData> {
        let arguments = arguments.unwrap_or_default();
        match self {
            Self::WorkingDirectory if arguments.is_empty() => Ok(PathBuf::from(".")),
            Self::WorkingDirectory => {
                Err(ErrorData::invalid_params("unexpected tool arguments", None))
            }
            Self::ToolArgument => {
                let parsed: ToolArguments = serde_json::from_value(Value::Object(arguments))
                    .map_err(|_| {
                        ErrorData::invalid_params("expected a project_directory argument", None)
                    })?;
                if !parsed.project_directory.is_absolute() {
                    return Err(ErrorData::invalid_params(
                        "project_directory must be an absolute path",
                        None,
                    ));
                }
                Ok(parsed.project_directory)
            }
        }
    }

    fn tool(
        &self,
        name: &'static str,
        description: &'static str,
        mut properties: Map<String, Value>,
        mut required: Vec<&str>,
    ) -> Tool {
        if matches!(self, Self::ToolArgument) {
            properties.insert("project_directory".into(), json!({
                "type": "string",
                "description": "Absolute path to the active project. The plugin starts outside the project."
            }));
            required.push("project_directory");
        }
        Tool::new(
            name,
            description,
            Map::from_iter([
                ("type".into(), json!("object")),
                ("properties".into(), json!(properties)),
                ("required".into(), json!(required)),
                ("additionalProperties".into(), json!(false)),
            ]),
        )
    }

    fn search(&self, arguments: Option<Map<String, Value>>) -> Result<CallToolResult, ErrorData> {
        let mut arguments = arguments.unwrap_or_default();
        let id = take_string(&mut arguments, "id")?;
        let query = take_string(&mut arguments, "query")?;
        let directory = self.resolve(Some(arguments))?;
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        if let Err(error) = output::replay(&directory, &id, Some(&query), &mut stdout, &mut stderr)
        {
            return Ok(CallToolResult::error(vec![ContentBlock::text(
                error.to_string(),
            )]));
        }
        let matches = [("stdout", stdout), ("stderr", stderr)]
            .into_iter()
            .filter(|(_, bytes)| !bytes.is_empty())
            .map(|(stream, bytes)| {
                ContentBlock::text(format!("{stream}:\n{}", String::from_utf8_lossy(&bytes)))
            })
            .collect();
        Ok(CallToolResult::success(matches))
    }
}

fn take_string(arguments: &mut Map<String, Value>, key: &str) -> Result<String, ErrorData> {
    match arguments.remove(key) {
        Some(Value::String(value)) => Ok(value),
        _ => Err(ErrorData::invalid_params(
            format!("{key} must be a string"),
            None,
        )),
    }
}

impl ServerHandler for ProjectDirectory {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
    }

    async fn list_tools(
        &self,
        _: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult::with_all_items(vec![
            self.tool(
                RUN_TOOL,
                "Run the project's configured Latch turn-end commands now and return their results. Commands may modify the workspace.",
                Map::new(),
                vec![],
            ),
            self.tool(
                SEARCH_TOOL,
                "Find omitted details in saved Latch failure output without rerunning the check. Copy the Saved log path from the failure report.",
                Map::from_iter([
                    ("id".into(), json!({"type": "string", "description": "Saved log path from the failure report, filename, or output ID beginning with r-."})),
                    ("query".into(), json!({"type": "string", "description": "A literal, case-sensitive term to search for in the saved output."})),
                ]),
                vec!["id", "query"],
            ).with_annotations(ToolAnnotations::new().read_only(true).open_world(false)),
            self.tool(
                STATUS_TOOL,
                "Show resolved project checks and latest automatic hook activity without running commands.",
                Map::new(),
                vec![],
            ).with_annotations(ToolAnnotations::new().read_only(true).open_world(false)),
        ]))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        if request.name == SEARCH_TOOL {
            return self.search(request.arguments).map(Into::into);
        }
        if request.name == STATUS_TOOL {
            let directory = self.resolve(request.arguments)?;
            return Ok(CallToolResult::success(vec![ContentBlock::text(format!(
                "{:#}",
                status::snapshot(&directory)
            ))])
            .into());
        }
        if request.name != RUN_TOOL {
            return Err(ErrorData::invalid_params("unknown Latch tool", None));
        }
        let directory = self.resolve(request.arguments)?;
        output::prune_if_due();
        let result = match config::load(&directory, config::HookName::TurnEnd) {
            Ok(Some(entries)) => {
                let report = run::run(&directory, entries.into_vec());
                let message = report.message(&directory);
                if report.failed() {
                    CallToolResult::error(vec![ContentBlock::text(message)])
                } else if message.is_empty() {
                    CallToolResult::success(vec![])
                } else {
                    CallToolResult::success(vec![ContentBlock::text(message)])
                }
            }
            Ok(None) => {
                CallToolResult::error(vec![ContentBlock::text("hooks.turn_end is not configured")])
            }
            Err(error) => CallToolResult::error(vec![ContentBlock::text(error.to_string())]),
        };
        Ok(result.into())
    }
}

struct StdioTransport(AsyncRwTransport<RoleServer, tokio::io::Stdin, tokio::io::Stdout>);

impl Transport<RoleServer> for StdioTransport {
    type Error = std::io::Error;

    fn send(
        &mut self,
        message: ServerJsonRpcMessage,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send + 'static {
        self.0.send(message)
    }

    async fn receive(&mut self) -> Option<ClientJsonRpcMessage> {
        loop {
            let message = self.0.receive().await?;
            // Antigravity can send this before initialized. Latch uses an explicit project
            // directory or the server working directory, so client roots do not apply.
            if !matches!(
                &message,
                JsonRpcMessage::Notification(notification)
                    if matches!(notification.notification, ClientNotification::RootsListChangedNotification(_))
            ) {
                return Some(message);
            }
        }
    }

    async fn close(&mut self) -> Result<(), Self::Error> {
        self.0.close().await
    }
}

pub(crate) fn dispatch(project: ProjectDirectory) -> ExitCode {
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()?
            .block_on(async {
                let (input, output) = rmcp::transport::stdio();
                project
                    .serve(StdioTransport(AsyncRwTransport::new(input, output)))
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
