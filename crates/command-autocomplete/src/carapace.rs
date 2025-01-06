use crate::connection::{ConnRequest, SendError, Transport};
use crate::types::{CompleteParams, CompleteResult, CompletionValue, Error, ShutdownResult};
use clap::Args;
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Args)]
pub struct CarapaceArgs {}

pub fn run_carapace(_args: CarapaceArgs) -> anyhow::Result<()> {
    let (transport, join_handle) = Transport::stdio();
    {
        let (_, receiver) = crate::connection::new_connection(transport);
        while let Some(req) = receiver.next_request() {
            match handle_request(req) {
                Ok(LoopAction::Continue) => continue,
                Ok(LoopAction::Stop) => break,
                Err(_) => {
                    log::warn!("the connection closed unexpectedly, stopping the receving loop");
                    break;
                }
            }
        }
    }
    join_handle.join()?;
    Ok(())
}
enum LoopAction {
    Continue,
    Stop,
}

fn handle_request(req: ConnRequest) -> Result<LoopAction, SendError> {
    match req.inner().method.as_str() {
        "complete" => match serde_json::from_value(req.inner().params.clone()) {
            Ok(params) => {
                req.reply(handle_complete_request(params))?;
            }
            Err(err) => {
                req.reply_err(Error::invalid_request(format!(
                    "invalid params for complete request: {err}"
                )))?;
            }
        },
        "shutdown" => {
            req.reply_ok(ShutdownResult {})?;
            return Ok(LoopAction::Stop);
        }
        _ => {
            let method = req.inner().method.clone();
            req.reply_err(Error {
                code: "UNKNOWN_REQUEST".to_string(),
                message: format!("method {} is not recognized", method),
            })?;
        }
    }
    Ok(LoopAction::Continue)
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
