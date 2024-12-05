use crate::sandbox::{Action, AnnotatorConfig};
use crate::{metrics, parse_action, parse_release, parse_runtime, sandbox::{self, Sandbox}, Error, ExecutionSnafu, Result, SandboxCreationSnafu, WebSocketTaskPanicSnafu, NullAwayConfigData};
use axum::extract::ws::{Message, WebSocket};
use snafu::prelude::*;
use std::{
    convert::{TryFrom, TryInto},
    time::Instant,
};
use tokio::{sync::mpsc, task::JoinSet};

type Meta = serde_json::Value;

#[derive(serde::Deserialize)]
#[serde(tag = "type")]
enum HandshakeMessage {
    #[serde(rename = "websocket/connected")]
    Connected {
        payload: Connected,
        #[allow(unused)]
        meta: Meta,
    },
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Connected {
    i_accept_this_is_an_unsupported_api: bool,
}

#[derive(serde::Deserialize)]
#[serde(tag = "type")]
enum WSMessageRequest {
    #[serde(rename = "output/execute/wsExecuteRequest")]
    ExecuteRequest { payload: ExecuteRequest, meta: Meta },
}

#[derive(serde::Deserialize)]
struct ExecuteRequest {
    runtime: String,
    release: String,
    action: String,
    code: String,
    preview: bool,
    nullaway_config_data: Option<NullAwayConfigData>,
    annotator_config: Option<AnnotatorConfig>,
}

impl TryFrom<ExecuteRequest> for sandbox::ExecuteRequest {
    type Error = Error;

    fn try_from(value: ExecuteRequest) -> Result<Self, Self::Error> {
        let ExecuteRequest {
            runtime,
            release,
            action,
            code,
            preview,
            nullaway_config_data: config_data,
            annotator_config: annotator_config,
        } = value;

        Ok(sandbox::ExecuteRequest {
            runtime: parse_runtime(&runtime)?,
            release: parse_release(&release)?,
            action: parse_action(&action)?.unwrap_or(Action::Run),
            preview,
            code,
            nullaway_config_data: config_data.map(|data| sandbox::NullAwayConfigData {
                cast_to_non_null_method: data.cast_to_non_null_method,
                check_optional_emptiness: data.check_optional_emptiness,
                check_contracts: data.check_contracts,
                j_specify_mode: data.j_specify_mode,
            }),
            annotator_config: annotator_config.map(|data| sandbox::AnnotatorConfig {
                nullUnmarked: data.nullUnmarked,
            }),
        })
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(tag = "type")]
enum MessageResponse {
    #[serde(rename = "websocket/error")]
    Error { payload: WSError, meta: Meta },

    #[serde(rename = "output/execute/wsExecuteResponse")]
    ExecuteResponse {
        payload: ExecuteResponse,
        meta: Meta,
    },
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct WSError {
    error: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ExecuteResponse {
    success: bool,
    stdout: String,
    stderr: String,
}

impl From<sandbox::ExecuteResponse> for ExecuteResponse {
    fn from(value: sandbox::ExecuteResponse) -> Self {
        let sandbox::ExecuteResponse {
            success,
            stdout,
            stderr,
        } = value;

        ExecuteResponse {
            success,
            stdout,
            stderr,
        }
    }
}

pub async fn handle(socket: WebSocket) {
    metrics::LIVE_WS.inc();
    let start = Instant::now();

    handle_core(socket).await;

    metrics::LIVE_WS.dec();
    let elapsed = start.elapsed();
    metrics::DURATION_WS.observe(elapsed.as_secs_f64());
}

async fn handle_core(mut socket: WebSocket) {
    if !connect_handshake(&mut socket).await {
        return;
    }

    let (tx, mut rx) = mpsc::channel(3);
    let mut tasks = JoinSet::new();

    // TODO: Implement some kind of timeout to shutdown running work?

    loop {
        tokio::select! {
            request = socket.recv() => {
                match request {
                    None => {
                        // browser disconnected
                        break;
                    }
                    Some(Ok(Message::Text(txt))) => handle_msg(txt, &tx, &mut tasks).await,
                    Some(Ok(_)) => {
                        // unknown message type
                        continue;
                    }
                    Some(Err(e)) => super::record_websocket_error(e.to_string()),
                }
            },
            resp = rx.recv() => {
                let resp = resp.expect("The rx should never close as we have a tx");
                let resp = resp.unwrap_or_else(error_to_response);
                let resp = response_to_message(resp);

                if let Err(_) = socket.send(resp).await {
                    // We can't send a response
                    break;
                }
            },
            // We don't care if there are no running tasks
            Some(task) = tasks.join_next() => {
                let Err(error) = task else { continue };
                // The task was cancelled; no need to report
                let Ok(panic) = error.try_into_panic() else { continue };

                let text = match panic.downcast::<String>() {
                    Ok(text) => *text,
                    Err(panic) => match panic.downcast::<&str>() {
                        Ok(text) => text.to_string(),
                        _ => "An unknown panic occurred".into(),
                    }
                };
                let error = WebSocketTaskPanicSnafu { text }.build();

                let resp = error_to_response(error);
                let resp = response_to_message(resp);

                if let Err(_) = socket.send(resp).await {
                    // We can't send a response
                    break;
                }
            },
        }
    }

    drop((tx, rx, socket));
    tasks.shutdown().await;
}

async fn connect_handshake(socket: &mut WebSocket) -> bool {
    let Some(Ok(Message::Text(txt))) = socket.recv().await else {
        return false;
    };
    let Ok(HandshakeMessage::Connected { payload, .. }) =
        serde_json::from_str::<HandshakeMessage>(&txt)
    else {
        return false;
    };
    if !payload.i_accept_this_is_an_unsupported_api {
        return false;
    }
    socket.send(Message::Text(txt)).await.is_ok()
}

fn error_to_response(error: Error) -> MessageResponse {
    let error = error.to_string();
    // TODO: thread through the Meta from the originating request
    let meta = serde_json::json!({ "sequenceNumber": -1 });
    MessageResponse::Error {
        payload: WSError { error },
        meta,
    }
}

fn response_to_message(response: MessageResponse) -> Message {
    const LAST_CHANCE_ERROR: &str =
        r#"{ "type": "WEBSOCKET_ERROR", "error": "Unable to serialize JSON" }"#;
    let resp = serde_json::to_string(&response).unwrap_or_else(|_| LAST_CHANCE_ERROR.into());
    Message::Text(resp)
}

async fn handle_msg(
    txt: String,
    tx: &mpsc::Sender<Result<MessageResponse>>,
    tasks: &mut JoinSet<Result<()>>,
) {
    use WSMessageRequest::*;

    let msg = serde_json::from_str(&txt).context(crate::DeserializationSnafu);

    match msg {
        Ok(ExecuteRequest { payload, meta }) => {
            let tx = tx.clone();
            tasks.spawn(async move {
                let resp = handle_execute(payload).await;
                let resp = resp.map(|payload| MessageResponse::ExecuteResponse { payload, meta });
                tx.send(resp).await.ok(/* We don't care if the runtime is closed */);
                Ok(())
            });
        }
        Err(e) => {
            let resp = Err(e);
            tx.send(resp).await.ok(/* We don't care if the runtime is closed */);
        }
    }
}

async fn handle_execute(req: ExecuteRequest) -> Result<ExecuteResponse> {
    let sb = Sandbox::new().await.context(SandboxCreationSnafu)?;

    let req = req.try_into()?;
    let resp = sb.execute(&req).await.context(ExecutionSnafu)?;
    Ok(resp.into())
}
