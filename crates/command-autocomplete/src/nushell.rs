use crate::connection::{ConnectionSender, Transport};
use crate::types::{CompleteParams, CompleteResult, Error};
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

    let stdin = child.stdin.take().context("missing stdin")?;
    let stdout = child.stdout.take().context("missing stdout")?;

    let (transport, join_handle) = Transport::raw(stdout, stdin);
    let (sender, receiver) = crate::connection::new_connection(transport);

    let recv_join_handle = std::thread::spawn(move || {
        // This is required to read the incoming responses.
        while let Some(req) = receiver.next_request() {
            let r = req.reply_err(Error::invalid_request("no requests expected"));
            if r.is_err() {
                log::warn!("The connection closed unexpectedly, stopping the receving loop");
                break;
            }
        }
    });
    if let Err(err) = complete_and_shutdown(args, sender) {
        log::error!("Completion failed, will kill subprocess: {}", err);
        if let Err(err) = child.kill() {
            log::warn!("Failed to kill subprocess: {err}")
        }
    }

    if let Err(err) = child.wait() {
        log::warn!("Failed to wait for the subprocess: {err}");
    }
    if let Err(err) = recv_join_handle.join() {
        log::warn!("receiving thread failed: {:?}", err);
    }
    if let Err(err) = join_handle.join() {
        log::warn!("connection threads failed: {:?}", err);
    }
    Ok(())
}

fn complete_and_shutdown(args: NushellArgs, sender: ConnectionSender) -> anyhow::Result<()> {
    // TODO: handle unwrap
    let res_handle = sender
        .send(
            "complete",
            CompleteParams {
                args: args.command.clone(),
            },
        )
        .context("complete command failed")?;

    // TODO: handle unwrap
    let result: CompleteResult = res_handle.wait().context("complete command failed")?;

    // TODO: handle unwrap
    sender.shutdown().unwrap().wait().unwrap();

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
