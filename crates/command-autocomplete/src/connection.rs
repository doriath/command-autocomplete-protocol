use crate::types::{Message, Request, RequestId, Response};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::sync::mpsc::{Receiver, SyncSender};
use std::sync::{Arc, Mutex};

// TODO: make it a trait
#[derive(Default)]
pub struct IdGenerator {
    next: Mutex<i32>,
}

impl IdGenerator {
    pub fn next(&self) -> RequestId {
        let mut x = self.next.lock().unwrap();
        let id = RequestId(format!("{}", x));
        *x += 1;
        id
    }
}

struct ResponseCallback {
    callback: Box<dyn FnOnce(Response) + Send + 'static>,
    shutdown: bool,
}

// Internal state of the connection
#[derive(Default)]
struct ConnectionState {
    responses: Mutex<HashMap<RequestId, ResponseCallback>>,
}

#[derive(Clone)]
pub struct ConnectionSender {
    ids: Arc<IdGenerator>,
    state: Arc<ConnectionState>,
    sender: SyncSender<Message>,
}

// shutdown
// - send shutdown request
// - disallow sending new requests
// - close connection

// when A sends shutdown
// - A can't send any new requests
// - when A receives response to shutdown, no new messages should be received
// - A can close receiver and sender

// when B receives shutdown
// - it knows it will not receive any new requests
// - it should respond to any active requests
// - when all active requests are responded to, and no new requests are coming
//   it should respond to the 'shutdown'
// - B can close receiver and sender

// premature shutdown

pub struct ResponseHandle<R> {
    receiver: Receiver<Result<R, ResponseError>>,
}

#[derive(Debug)]
pub enum ResponseError {
    /// Error received by the other side.
    Err(crate::types::Error),
    /// The connection has been closed and response will not be received.
    ChannelClosed,
    /// The received response failed deserialization into provided type.
    DeserializationError(serde_json::Error),
}

impl<R> ResponseHandle<R> {
    pub fn wait(self) -> Result<R, ResponseError> {
        match self.receiver.recv() {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(ResponseError::ChannelClosed),
        }
    }
}

impl std::fmt::Display for ResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResponseError::Err(err) => write!(f, "received error: {:?}", err),
            ResponseError::ChannelClosed => {
                write!(f, "response not received, channel has been closed")
            }
            ResponseError::DeserializationError(err) => {
                write!(f, "response has unexpected result type: {err}")
            }
        }
    }
}

impl std::error::Error for ResponseError {}

impl ConnectionSender {
    /// Sends the request to the other side of the connection.
    ///
    /// Returns a ResponseHandle, that will return a response when received.
    /// Note that for the response to be received, the ConnectionReceiver has to
    /// be continuously looped over for new requests.
    ///
    /// If the connection is already closed, error is returned.
    pub fn send<R: for<'a> Deserialize<'a> + 'static + Send>(
        &self,
        method: impl Into<String>,
        params: impl Serialize,
    ) -> Result<ResponseHandle<R>, SendError> {
        let id = self.ids.next();

        let method: String = method.into();
        let shutdown = method == "shutdown";

        self.sender
            .send(Request::new(id.clone(), method, params).into())
            .map_err(|_| SendError {})?;

        let (tx, rx) = std::sync::mpsc::sync_channel(0);

        let callback = ResponseCallback {
            callback: Box::new(move |response: Response| {
                let r: Result<R, ResponseError> = match response {
                    Response::Ok { id: _, result } => {
                        serde_json::from_value(result).map_err(ResponseError::DeserializationError)
                    }
                    Response::Err { id: _, error } => Err(ResponseError::Err(error)),
                };
                if tx.send(r).is_err() {
                    log::debug!("response ignored, response handle was dropped");
                }
            }),
            shutdown,
        };

        self.state.responses.lock().unwrap().insert(id, callback);
        Ok(ResponseHandle { receiver: rx })
    }

    /// Sends shutdown request to the other side.
    ///
    /// No new requests are allowed to be send after this call.
    pub fn shutdown(self) -> Result<ResponseHandle<serde_json::Value>, SendError> {
        self.send("shutdown", json!({}))
    }
}

pub struct ConnectionReceiver {
    state: Arc<ConnectionState>,
    receiver: Receiver<Message>,
    sender: SyncSender<Message>,
    shutdown: Mutex<bool>,
}

pub struct ConnRequest {
    inner: Request,
    sender: SyncSender<Message>,
}

impl ConnRequest {
    pub fn inner(&self) -> &Request {
        &self.inner
    }

    pub fn reply<R: Serialize>(
        self,
        response: Result<R, crate::types::Error>,
    ) -> Result<(), SendError> {
        match response {
            Ok(result) => self.reply_ok(result),
            Err(err) => self.reply_err(err),
        }
    }

    pub fn reply_ok<R: Serialize>(self, result: R) -> Result<(), SendError> {
        let response = Response::new_ok(self.inner.id, result);
        self.sender
            .send(Message::Response(response))
            .map_err(|_| SendError {})
    }
    pub fn reply_err(self, err: crate::types::Error) -> Result<(), SendError> {
        let response = Response::new_err(self.inner.id, err);
        self.sender
            .send(Message::Response(response))
            .map_err(|_| SendError {})
    }
}

impl ConnectionReceiver {
    // Note: This has to be called / polled continuously to ensure the
    // responses are populated
    // returns None when the connection is closed
    pub fn next_request(&self) -> Option<ConnRequest> {
        if *self.shutdown.lock().unwrap() {
            return None;
        }
        while let Ok(msg) = self.receiver.recv() {
            match msg {
                Message::Request(req) => {
                    return Some(ConnRequest {
                        inner: req,
                        sender: self.sender.clone(),
                    })
                }
                Message::Response(res) => {
                    let mut r = self.state.responses.lock().unwrap();
                    let Some(callback) = r.remove(res.id()) else {
                        log::warn!(
                            "Received response for id {:?}, but such request was never sent",
                            res.id()
                        );
                        return None;
                    };
                    (callback.callback)(res);
                    if callback.shutdown {
                        let mut x = self.shutdown.lock().unwrap();
                        *x = true;
                        return None;
                    }
                }
            }
        }
        None
    }
}

pub fn new_connection(transport: Transport) -> (ConnectionSender, ConnectionReceiver) {
    let state = Arc::new(ConnectionState::default());
    (
        ConnectionSender {
            ids: Default::default(),
            state: state.clone(),
            sender: transport.sender.clone(),
        },
        ConnectionReceiver {
            state,
            receiver: transport.receiver,
            sender: transport.sender,
            shutdown: Default::default(),
        },
    )
}

pub struct Transport {
    receiver: Receiver<Message>,
    sender: SyncSender<Message>,
}

#[derive(Debug)]
pub struct SendError {}

impl std::fmt::Display for SendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "failed to send a message, the channel is already closed")
    }
}

impl std::error::Error for SendError {}

pub struct JoinHandle {
    read_join: std::thread::JoinHandle<()>,
    write_join: std::thread::JoinHandle<()>,
}

impl JoinHandle {
    pub fn join(self) -> anyhow::Result<()> {
        self.read_join.join().unwrap();
        self.write_join.join().unwrap();
        Ok(())
    }
}

impl Transport {
    pub fn stdio() -> (Transport, JoinHandle) {
        Self::raw(std::io::stdin(), std::io::stdout())
    }

    pub fn raw<R: Read + Send + 'static, W: Write + Send + 'static>(
        read: R,
        write: W,
    ) -> (Transport, JoinHandle) {
        let (read_tx, read_rx) = std::sync::mpsc::sync_channel(0);
        let read_join = std::thread::spawn(move || {
            if let Err(err) = read_loop(read, read_tx) {
                log::error!("read_loop err: {err}");
            }
        });
        let (write_tx, write_rx) = std::sync::mpsc::sync_channel(0);
        let write_join = std::thread::spawn(move || {
            if let Err(err) = write_loop(write, write_rx) {
                log::error!("write_loop err: {err}");
            }
        });
        (
            Transport {
                receiver: read_rx,
                sender: write_tx,
            },
            JoinHandle {
                read_join,
                write_join,
            },
        )
    }

    pub fn send(&self, message: Message) -> Result<(), SendError> {
        self.sender.send(message).map_err(|_| SendError {})
    }

    // TODO: should Iterator be used here?
    // TODO: should Result be returned, to differentiate error from
    // cleanly closed channel?
    pub fn next_message(&self) -> Option<Message> {
        self.receiver.recv().ok()
    }
}

fn read_loop<R: Read>(read: R, sender: SyncSender<Message>) -> anyhow::Result<()> {
    let reader = BufReader::new(read);
    for line in reader.lines() {
        let msg: Message = serde_json::from_str(&line?)?;
        log::trace!("received: {:?}", msg);
        sender.send(msg)?;
    }
    log::debug!("read_loop: finished");
    Ok(())
}

fn write_loop<W: Write>(mut write: W, receiver: Receiver<Message>) -> anyhow::Result<()> {
    loop {
        let Ok(msg) = receiver.recv() else {
            break;
        };
        log::trace!("sending: {:?}", msg);
        let mut b = serde_json::to_vec(&msg)?;
        b.push(b'\n');
        write.write_all(&b)?;
        write.flush()?;
    }
    log::debug!("write_loop: finished");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use googletest::prelude::*;
    use serde_json::json;
    use std::{collections::VecDeque, io::Cursor, sync::mpsc::Sender};
    use test_log::test;

    struct PipeRead {
        state: VecDeque<u8>,
        receiver: Receiver<Vec<u8>>,
    }

    impl Read for PipeRead {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.state.is_empty() {
                if let Ok(v) = self.receiver.recv() {
                    self.state.extend(&v);
                }
            }
            self.state.read(buf)
        }
    }

    struct PipeWrite {
        state: Vec<u8>,
        sender: Sender<Vec<u8>>,
    }

    impl Write for PipeWrite {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.state.write(buf)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            let mut val = vec![];
            std::mem::swap(&mut self.state, &mut val);
            if !val.is_empty() {
                self.sender
                    .send(val)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::UnexpectedEof, e))?;
            }
            Ok(())
        }
    }

    impl Drop for PipeWrite {
        fn drop(&mut self) {
            let mut val = vec![];
            std::mem::swap(&mut self.state, &mut val);
            if !val.is_empty() {
                // TODO: handle unwrap
                self.sender.send(val).unwrap();
            }
        }
    }

    fn pipe() -> (PipeWrite, PipeRead) {
        let (tx, rx) = std::sync::mpsc::channel();
        (
            PipeWrite {
                state: Default::default(),
                sender: tx,
            },
            PipeRead {
                state: Default::default(),
                receiver: rx,
            },
        )
    }

    #[test(gtest)]
    fn reads_one_message() {
        let input =
            serde_json::to_vec(&json!({"id": "1", "method": "complete", "params":{}})).unwrap();
        let c = Cursor::new(input);
        let output: Vec<u8> = Vec::new();
        let (t, join_handles) = Transport::raw(c, output);
        expect_that!(t.next_message(), some(anything()));
        expect_that!(t.next_message(), none());
        join_handles.join().unwrap();
    }

    #[test(gtest)]
    fn writes_one_message() {
        let (pipe_w, mut pipe_r) = pipe();
        let c = Cursor::new(vec![]);
        let (t, join_handles) = Transport::raw(c, pipe_w);
        let response = Message::Response(Response::new_err(
            RequestId("1".into()),
            crate::types::Error::internal("test"),
        ));
        t.send(response.clone()).unwrap();
        // Drop, to ensure that the pipe is closed (otherwise below read_to_end will never finish).
        drop(t);
        let mut output = vec![];
        pipe_r.read_to_end(&mut output).unwrap();
        let mut expected = serde_json::to_vec(&response).unwrap();
        expected.push(b'\n');
        expect_that!(output, eq(&expected));
        join_handles.join().unwrap();
    }
}
