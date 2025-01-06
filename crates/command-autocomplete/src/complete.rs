use crate::{
    connection::{ConnRequest, SendError, Transport},
    types::{CompleteParams, CompleteResult, CompletionValue, Error, ShutdownResult},
};

pub fn run_complete() -> anyhow::Result<()> {
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
    let mut completions = vec![];
    if params.args.len() == 2 {
        completions = vec![
            CompletionValue {
                value: "shell ".into(),
                description: None,
            },
            CompletionValue {
                value: "router ".into(),
                description: None,
            },
            CompletionValue {
                value: "bridge ".into(),
                description: None,
            },
            CompletionValue {
                value: "complete ".into(),
                description: None,
            },
        ];
    }
    Ok(CompleteResult {
        values: completions,
    })
}
