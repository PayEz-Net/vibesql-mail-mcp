use actix_web::web::Bytes;
use actix_web::{web, HttpResponse};
use futures::stream::Stream;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Mutex;
use std::task::{Context, Poll};
use tokio::sync::mpsc;

pub struct SseClient {
    rx: mpsc::UnboundedReceiver<String>,
}

impl Stream for SseClient {
    type Item = Result<Bytes, actix_web::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.rx.poll_recv(cx) {
            Poll::Ready(Some(msg)) => Poll::Ready(Some(Ok(Bytes::from(msg)))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub struct SseBroadcaster {
    clients: Mutex<HashMap<String, Vec<mpsc::UnboundedSender<String>>>>,
}

impl SseBroadcaster {
    pub fn new() -> Self {
        Self {
            clients: Mutex::new(HashMap::new()),
        }
    }

    pub fn subscribe(&self, agent: &str) -> SseClient {
        let (tx, rx) = mpsc::unbounded_channel();

        // Send connected event
        let connect_msg = format!(
            "event: connected\ndata: {}\n\n",
            serde_json::json!({"agent": agent})
        );
        let _ = tx.send(connect_msg);

        let mut clients = self.clients.lock().unwrap();
        clients
            .entry(agent.to_string())
            .or_insert_with(Vec::new)
            .push(tx);

        SseClient { rx }
    }

    pub fn send(&self, agent: &str, event: &str, data: &Value) {
        let msg = format!("event: {}\ndata: {}\n\n", event, data);
        let mut clients = self.clients.lock().unwrap();
        if let Some(senders) = clients.get_mut(agent) {
            senders.retain(|tx| tx.send(msg.clone()).is_ok());
        }
    }

    /// Send heartbeat to all clients
    pub fn heartbeat(&self) {
        let msg = ": heartbeat\n\n".to_string();
        let mut clients = self.clients.lock().unwrap();
        for senders in clients.values_mut() {
            senders.retain(|tx| tx.send(msg.clone()).is_ok());
        }
    }
}

#[derive(Deserialize)]
pub struct StreamQuery {
    pub agent: String,
}

/// GET /v1/mail/stream?agent=BAPert
pub async fn event_stream(
    broadcaster: web::Data<SseBroadcaster>,
    query: web::Query<StreamQuery>,
) -> HttpResponse {
    let client = broadcaster.subscribe(&query.agent);

    HttpResponse::Ok()
        .insert_header(("Content-Type", "text/event-stream"))
        .insert_header(("Cache-Control", "no-cache"))
        .insert_header(("Connection", "keep-alive"))
        .streaming(client)
}
