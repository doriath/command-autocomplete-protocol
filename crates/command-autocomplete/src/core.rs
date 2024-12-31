use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
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

impl From<Request> for Message {
    fn from(value: Request) -> Self {
        Message::Request(value)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub id: RequestId,
    pub method: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq, Hash)]
pub struct RequestId(pub String);

impl Request {
    pub fn new(id: RequestId, method: impl Into<String>, params: impl Serialize) -> Self {
        Request {
            id,
            method: method.into(),
            params: serde_json::to_value(params).unwrap(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Response {
    Ok {
        id: RequestId,
        result: serde_json::Value,
    },
    Err {
        id: RequestId,
        error: Error,
    },
}

impl Response {
    pub fn id(&self) -> &RequestId {
        match self {
            Response::Ok { id, result: _ } => id,
            Response::Err { id, error: _ } => id,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
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
    pub fn new_ok<R: Serialize>(id: RequestId, result: R) -> Self {
        Response::Ok {
            id,
            result: serde_json::to_value(result).unwrap(),
        }
    }
    pub fn new_err(id: RequestId, error: Error) -> Self {
        Response::Err { id, error }
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
