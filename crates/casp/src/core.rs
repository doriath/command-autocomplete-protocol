use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Message {
    Request(Request),
    Response(Response),
}

impl From<Response> for Message {
    fn from(value: Response) -> Self {
        Message::Response(value)
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub id: String,
    pub method: String,
    pub params: serde_json::Value,
}

impl Request {
    pub fn new(id: impl Into<String>, method: impl Into<String>, params: impl Serialize) -> Self {
        Request {
            id: id.into(),
            method: method.into(),
            params: serde_json::to_value(params).unwrap(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Response {
    Ok {
        id: String,
        result: serde_json::Value,
    },
    Err {
        id: String,
        error: Error,
    },
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Error {
    pub code: String,
    pub message: String,
}

impl Error {
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Error {
            code: "INVALID_REQUEST".into(),
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Error {
            code: "INTERNAL".into(),
            message: message.into(),
        }
    }
}

impl Response {
    pub fn new_ok<R: Serialize>(id: impl Into<String>, result: R) -> Self {
        Response::Ok {
            id: id.into(),
            result: serde_json::to_value(result).unwrap(),
        }
    }
    pub fn new_err(id: impl Into<String>, error: Error) -> Self {
        Response::Err {
            id: id.into(),
            error,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CompleteParams {
    pub args: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CompleteResult {
    pub values: Vec<CompletionValue>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CompletionValue {
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}
