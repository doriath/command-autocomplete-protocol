use crate::connection::Transport;
use crate::types::{CompleteParams, CompleteResult, CompletionValue, Error, Request, Response};
use clap::Args;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::process::Command;

#[derive(Debug, Args)]
pub struct CarapaceArgs {}

pub fn run_carapace(_args: CarapaceArgs) -> anyhow::Result<()> {
    let (transport, join_handles) = Transport::stdio();
    {
        let (_, receiver) = crate::connection::new_connection(transport);
        while let Some(req) = receiver.next_request() {
            if req.method == "shutdown" {
                receiver.reply(Response::new_ok(req.id, json!({})));
                break;
            }
            receiver.reply(handle_request(req));
        }
    }
    join_handles.join()?;
    Ok(())
}

fn handle_request(req: Request) -> Response {
    match req.method.as_str() {
        "complete" => {
            let Ok(params) = serde_json::from_value(req.params) else {
                return Response::new_err(
                    req.id,
                    Error::invalid_request("invalid params for complete request"),
                );
            };
            Response::new_ok(req.id, handle_complete_request(params).unwrap())
        }
        _ => Response::new_err(
            req.id,
            Error {
                code: "UNKNOWN_REQUEST".to_string(),
                message: format!("method {} is not recognized", req.method),
            },
        ),
    }
}

fn handle_complete_request(params: CompleteParams) -> Result<CompleteResult, Error> {
    if params.args.is_empty() {
        return Err(Error::invalid_request(
            "params.args is empty, required at least one element",
        ));
    }

    let mut args = Vec::new();
    args.push(params.args[0].clone());
    args.push("export".into());
    args.extend_from_slice(&params.args);
    let output = Command::new("carapace")
        .args(args)
        .output()
        .map_err(|e| Error::internal(format!("failed to run carapace command: {e}")))?;
    if !output.status.success() {
        return Err(Error::internal("carapace command failed"));
    }

    let carapace_export: CarapaceExport = serde_json::from_slice(&output.stdout)
        .map_err(|e| Error::internal(format!("output from carapace can't be parsed: {e}")))?;

    Ok(CompleteResult {
        values: carapace_export
            .values
            .into_iter()
            .map(|x| CompletionValue {
                value: x.value,
                description: x.description,
            })
            .collect(),
    })
}

#[derive(Debug, Deserialize, Serialize)]
struct CarapaceExport {
    pub values: Vec<CarapaceValue>,
}

#[derive(Debug, Deserialize, Serialize)]
struct CarapaceValue {
    pub value: String,
    pub display: Option<String>,
    pub description: Option<String>,
    pub tag: Option<String>,
}
