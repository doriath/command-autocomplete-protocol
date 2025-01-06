use crate::connection::{ConnRequest, ResponseError, SendError, Transport};
use crate::types::{CompleteParams, CompleteResult, Error, ShutdownResult};
use clap::Args;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;

#[derive(Debug, Args)]
pub struct RouterArgs {
    /// The configuration path for available completers.
    config: Option<PathBuf>,
    // TODO: add a config
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Completer {
    command: String,
    args: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Command {
    name: String,
    completer: Completer,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Config {
    pub command: Vec<Command>,
}

pub fn run_router(_args: RouterArgs) -> anyhow::Result<()> {
    let content = std::fs::read_to_string(
        Path::new(&std::env::var("HOME")?).join(".config/command-autocomplete/completers.toml"),
    );
    let config = match content {
        Ok(content) => toml::from_str(&content)?,
        // TODO: check the error, only return default on not found
        Err(_) => Config::default(),
    };
    let (transport, join_handle) = Transport::stdio();
    {
        let (_, receiver) = crate::connection::new_connection(transport);
        let mut router = Router::new(config);
        while let Some(req) = receiver.next_request() {
            match router.handle_request(req) {
                Ok(LoopAction::Continue) => continue,
                Ok(LoopAction::Stop) => break,
                Err(_) => {
                    log::warn!("the connection closed unexpectedly, stopping the receving loop");
                    break;
                }
            };
        }
    }
    join_handle.join()?;
    Ok(())
}

struct Router {
    config: Config,
}

enum LoopAction {
    Continue,
    Stop,
}

impl Router {
    fn new(config: Config) -> Self {
        Router { config }
    }

    fn handle_request(&mut self, req: ConnRequest) -> Result<LoopAction, SendError> {
        match req.inner().method.as_str() {
            "complete" => match serde_json::from_value(req.inner().params.clone()) {
                Ok(params) => {
                    req.reply(self.handle_complete_request(params))?;
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

    fn completer(&self, params: &CompleteParams) -> Option<std::process::Command> {
        if params.args.is_empty() {
            return None;
        }
        for command in &self.config.command {
            if command.name != params.args[0] {
                continue;
            }
            let mut cmd = std::process::Command::new(&command.completer.command);
            cmd.args(&command.completer.args);
            return Some(cmd);
        }
        None
    }

    fn handle_complete_request(&mut self, params: CompleteParams) -> Result<CompleteResult, Error> {
        let Some(mut command) = self.completer(&params) else {
            if !params.args.is_empty() {
                log::info!("completer for command {} not found", params.args[0]);
            }
            // TODO: handle this better
            return Ok(CompleteResult { values: vec![] });
        };
        log::debug!("starting external completer: {:?}", command);

        // TODO: unwrap
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| Error::internal(format!("failed to start the completer: {e}")))?;

        // TODO: unwrap
        let stdin = child.stdin.take().ok_or_else(|| {
            Error::internal("stdin missing in started process, this should never happen")
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            Error::internal("stdout missing in started process, this should never happen")
        })?;

        let (transport, join_handle) = Transport::raw(stdout, stdin);
        let (sender, receiver) = crate::connection::new_connection(transport);

        let recv_join_handle = std::thread::spawn(move || {
            // ensuring we read the response
            while let Some(req) = receiver.next_request() {
                let r = req.reply_err(Error::invalid_request("no requests expected"));
                if r.is_err() {
                    break;
                }
            }
            log::debug!("receiver finished");
        });
        // TODO: unwrap
        log::debug!("sending complete request to sub process");
        let res = sender.send::<CompleteResult>("complete", params).unwrap();
        log::debug!("waiting for complete response");
        let res = res.wait().map_err(|e| match e {
            ResponseError::Err(e) => e,
            ResponseError::ChannelClosed => {
                Error::internal("subprocess closed connection before providing completions")
            }
            ResponseError::DeserializationError(err) => Error::internal(format!(
                "subprocess returned response that failed deserialization, error: {err}"
            )),
        });
        log::debug!("received response: {:?}", res.is_ok());

        // TODO: handle unwrap
        sender.shutdown().unwrap().wait().unwrap();

        join_handle.join().unwrap();
        recv_join_handle.join().unwrap();
        child.wait().unwrap();

        // TODO: exit cleanly
        res
    }
}
