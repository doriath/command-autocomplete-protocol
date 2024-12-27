use crate::core::{
    CompleteParams, CompleteResult, CompletionValue, Error, Message, Request, Response,
};
use clap::Args;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::process::Command;

#[derive(Debug, Args)]
pub struct CarapaceArgs {}

pub fn run_carapace(_args: CarapaceArgs) -> anyhow::Result<()> {
    let stdin = std::io::stdin();
    let reader = BufReader::new(stdin);

    for line in reader.lines() {
        let msg: Message = serde_json::from_str(&line?)?;
        let response = handle_message(msg)?;
        println!("{}", serde_json::to_string(&response)?);
        std::io::stdout().flush()?;
    }

    Ok(())
}

fn handle_message(msg: Message) -> anyhow::Result<Response> {
    let Message::Request(req) = msg else {
        anyhow::bail!("received a message that is not a request which is not supported");
    };
    Ok(handle_request(req))
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
