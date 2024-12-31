use crate::core::{CompleteParams, CompleteResult, Message, Request, RequestId, Response};
use anyhow::Context;
use clap::Args;
use serde_json::json;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

#[derive(Debug, Args)]
pub struct NushellArgs {
    /// args of the command that is being completed
    #[arg(last = true)]
    command: Vec<String>,
}

pub fn run_nushell(args: NushellArgs) -> anyhow::Result<()> {
    // TODO: make it customizable (we should actually invoke a router)
    let mut child = Command::new("command-autocomplete")
        .args(["router"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    // Send one message to perform completion
    let mut stdin = child.stdin.take().context("missing stdin")?;
    let stdout = child.stdout.context("missing stdout")?;
    let req = Request::new(
        RequestId("1".into()),
        "complete",
        CompleteParams {
            args: args.command.clone(),
        },
    );
    let mut b = serde_json::to_vec(&req)?;
    b.push(b'\n');
    stdin.write_all(&b)?;
    stdin.flush()?;
    drop(stdin);

    // Read the result of the completion.
    let reader = BufReader::new(stdout);
    for line in reader.lines() {
        let msg: Message = serde_json::from_str(&line?)?;
        let Message::Response(response) = msg else {
            anyhow::bail!("received message other than response");
        };
        let Response::Ok { id: _, result } = response else {
            anyhow::bail!("received error");
        };
        let result: CompleteResult = serde_json::from_value(result)?;
        println!(
            "{}",
            json!(result
                .values
                .into_iter()
                .map(|v| {
                    json! ({
                        "value": v.value,
                        "description": v.description,
                    })
                })
                .collect::<Vec<_>>())
        );
    }
    Ok(())
}
