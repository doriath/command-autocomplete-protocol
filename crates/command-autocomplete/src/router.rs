use crate::connection::{ResponseError, Transport};
use crate::types::{CompleteParams, CompleteResult, Error, Request, Response};
use clap::Args;
use serde::{Deserialize, Serialize};
use serde_json::json;
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
    let (transport, join_handles) = Transport::stdio();
    {
        let (_, receiver) = crate::connection::new_connection(transport);
        let mut router = Router::new(config);
        while let Some(req) = receiver.next_request() {
            if req.method == "shutdown" {
                receiver.reply(Response::new_ok(req.id, json!({})));
                break;
            }
            receiver.reply(router.handle_request(req));
        }
    }
    join_handles.join()?;
    Ok(())
}

struct Router {
    config: Config,
}

impl Router {
    fn new(config: Config) -> Self {
        Router { config }
    }

    fn handle_request(&mut self, req: Request) -> Response {
        match req.method.as_str() {
            "complete" => {
                let Ok(params) = serde_json::from_value(req.params) else {
                    return Response::new_err(
                        req.id,
                        Error::invalid_request("invalid params for complete request"),
                    );
                };
                Response::new_ok(req.id, self.handle_complete_request(params).unwrap())
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
        // TODO: unwrap
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        // TODO: unwrap
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();

        log::debug!("starting sub connection");
        let (transport, _join_handles) = Transport::raw(stdout, stdin);
        let (sender, receiver) = crate::connection::new_connection(transport);

        // TODO: join
        let _join = std::thread::spawn(move || {
            // ensuring we read the response
            log::debug!("receiver.next_request(): start");
            while let Some(req) = receiver.next_request() {
                receiver.reply(Response::new_err(
                    req.id,
                    Error::invalid_request("no requests expected"),
                ));
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

        // TODO: exit cleanly
        // child.wait().unwrap();
        res
    }
}
