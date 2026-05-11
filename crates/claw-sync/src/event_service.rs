use std::sync::Arc;

use tokio::sync::{broadcast, RwLock};
use tonic::{Request, Response, Status};

use claw_store::ClawStore;

use crate::proto::event::event_stream_service_server::EventStreamService;
use crate::proto::event::*;

#[derive(Debug, Clone)]
pub struct EventBus {
    tx: broadcast::Sender<Event>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity.max(1));
        Self { tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.tx.subscribe()
    }

    pub fn publish(&self, event: Event) {
        let _ = self.tx.send(event);
    }

    pub fn publish_ref_event(
        &self,
        event_type: &str,
        ref_name: impl Into<String>,
        object_id: Option<crate::proto::common::ObjectId>,
    ) {
        let ref_name = ref_name.into();
        self.publish(Event {
            event_type: event_type.to_string(),
            timestamp: now_ms(),
            ref_name: ref_name.clone(),
            object_id,
            message: format!("{event_type}: {ref_name}"),
        });
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(1_024)
    }
}

pub struct EventServer {
    bus: EventBus,
}

impl EventServer {
    pub fn new(_store: Arc<RwLock<ClawStore>>) -> Self {
        Self::with_bus(EventBus::default())
    }

    pub fn with_bus(bus: EventBus) -> Self {
        Self { bus }
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn matches_subscription(event: &Event, event_types: &[String], ref_prefix: &str) -> bool {
    (event_types.is_empty() || event_types.iter().any(|t| t == &event.event_type))
        && (ref_prefix.is_empty() || event.ref_name.starts_with(ref_prefix))
}

#[tonic::async_trait]
impl EventStreamService for EventServer {
    type SubscribeStream = tokio_stream::wrappers::ReceiverStream<Result<Event, Status>>;

    async fn subscribe(
        &self,
        request: Request<SubscribeRequest>,
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        let req = request.into_inner();
        let event_types = req.event_types;
        let ref_prefix = req.ref_prefix;
        let mut bus_rx = self.bus.subscribe();

        let (tx, rx) = tokio::sync::mpsc::channel(64);

        tokio::spawn(async move {
            loop {
                match bus_rx.recv().await {
                    Ok(event) => {
                        if matches_subscription(&event, &event_types, &ref_prefix)
                            && tx.send(Ok(event)).await.is_err()
                        {
                            return;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        let event = Event {
                            event_type: "event_stream_lagged".to_string(),
                            timestamp: now_ms(),
                            ref_name: ref_prefix.clone(),
                            object_id: None,
                            message: format!("event stream lagged by {skipped} messages"),
                        };
                        if matches_subscription(&event, &event_types, &ref_prefix)
                            && tx.send(Ok(event)).await.is_err()
                        {
                            return;
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => return,
                }
            }
        });

        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(
            rx,
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claw_core::hash::content_hash;
    use claw_core::object::TypeTag;
    use tokio_stream::StreamExt;

    #[tokio::test]
    async fn subscribe_streams_matching_bus_events() {
        let bus = EventBus::new(8);
        let server = EventServer::with_bus(bus.clone());

        let response = server
            .subscribe(Request::new(SubscribeRequest {
                event_types: vec!["ref_created".to_string()],
                ref_prefix: "heads/".to_string(),
            }))
            .await
            .unwrap();
        let mut stream = response.into_inner();

        let id = content_hash(TypeTag::Blob, b"blob");
        bus.publish_ref_event(
            "ref_created",
            "heads/main",
            Some(crate::proto::common::ObjectId {
                hash: id.as_bytes().to_vec(),
            }),
        );

        let event = tokio::time::timeout(std::time::Duration::from_secs(1), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();

        assert_eq!(event.event_type, "ref_created");
        assert_eq!(event.ref_name, "heads/main");
        assert_eq!(event.object_id.unwrap().hash, id.as_bytes().to_vec());
    }

    #[tokio::test]
    async fn subscribe_filters_event_type_and_ref_prefix() {
        let bus = EventBus::new(8);
        let server = EventServer::with_bus(bus.clone());

        let response = server
            .subscribe(Request::new(SubscribeRequest {
                event_types: vec!["ref_updated".to_string()],
                ref_prefix: "heads/".to_string(),
            }))
            .await
            .unwrap();
        let mut stream = response.into_inner();

        bus.publish_ref_event("ref_created", "heads/main", None);
        bus.publish_ref_event("ref_updated", "tags/v1", None);
        bus.publish_ref_event("ref_updated", "heads/main", None);

        let event = tokio::time::timeout(std::time::Duration::from_secs(1), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();

        assert_eq!(event.event_type, "ref_updated");
        assert_eq!(event.ref_name, "heads/main");
    }
}
