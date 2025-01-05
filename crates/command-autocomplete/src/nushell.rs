use crate::connection::Transport;
use crate::types::{CompleteParams, CompleteResult, Error, Response};
use anyhow::Context;
use clap::Args;
use serde_json::json;
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
    let stdin = child.stdin.take().context("missing stdin")?;
    let stdout = child.stdout.take().context("missing stdout")?;

    let (transport, join_handle) = Transport::raw(stdout, stdin);
    let (sender, receiver) = crate::connection::new_connection(transport);

    let recv_join_handle = std::thread::spawn(move || {
        // This is required to read the incoming responses.
        while let Some(req) = receiver.next_request() {
            receiver.reply(Response::new_err(
                req.id,
                Error::invalid_request("no requests expected"),
            ));
        }
    });
    // TODO: handle unwrap
    let res_handle = sender
        .send(
            "complete",
            CompleteParams {
                args: args.command.clone(),
            },
        )
        .unwrap();

    // TODO: handle unwrap
    let result: CompleteResult = res_handle.wait().unwrap();

    // TODO: handle unwrap
    sender.shutdown().unwrap().wait().unwrap();

    log::debug!("waiting for transport threads to finish");
    join_handle.join()?;
    log::debug!("waiting for receiver to finish");
    // TODO: handle unwrap
    recv_join_handle.join().unwrap();

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
    Ok(())
}
