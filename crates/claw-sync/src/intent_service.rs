use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::RwLock;
use tonic::{Request, Response, Status};

use claw_core::id::IntentId;
use claw_core::object::Object;
use claw_core::types::{Intent, IntentStatus};
use claw_store::ClawStore;

use crate::proto::intent::intent_service_server::IntentService;
use crate::proto::intent::*;
use crate::security::{AuditSink, AuthorizationAction, Authorizer, ServiceSecurity};

pub struct IntentServer {
    store: Arc<RwLock<ClawStore>>,
    security: ServiceSecurity,
}

impl IntentServer {
    pub fn new(store: Arc<RwLock<ClawStore>>) -> Self {
        Self {
            store,
            security: ServiceSecurity::default(),
        }
    }

    pub fn with_authorizer(mut self, authorizer: Arc<dyn Authorizer>) -> Self {
        self.security = self.security.with_authorizer(authorizer);
        self
    }

    pub fn with_audit_sink(mut self, audit_sink: Arc<dyn AuditSink>) -> Self {
        self.security = self.security.with_audit_sink(audit_sink);
        self
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn intent_to_proto(i: &Intent) -> crate::proto::objects::Intent {
    crate::proto::objects::Intent {
        id: Some(crate::proto::common::Ulid {
            data: i.id.as_bytes().to_vec(),
        }),
        title: i.title.clone(),
        goal: i.goal.clone(),
        constraints: i.constraints.clone(),
        acceptance_tests: i.acceptance_tests.clone(),
        links: i.links.clone(),
        policy_refs: i.policy_refs.clone(),
        agents: i.agents.clone(),
        change_ids: i.change_ids.clone(),
        depends_on: i.depends_on.clone(),
        supersedes: i.supersedes.clone(),
        status: match i.status {
            IntentStatus::Open => "open".into(),
            IntentStatus::Blocked => "blocked".into(),
            IntentStatus::Done => "done".into(),
            IntentStatus::Superseded => "superseded".into(),
        },
        created_at_ms: i.created_at_ms,
        updated_at_ms: i.updated_at_ms,
    }
}

#[allow(clippy::result_large_err)]
fn parse_status(s: &str) -> Result<IntentStatus, Status> {
    match s {
        "open" => Ok(IntentStatus::Open),
        "blocked" => Ok(IntentStatus::Blocked),
        "done" => Ok(IntentStatus::Done),
        "superseded" => Ok(IntentStatus::Superseded),
        _ => Err(Status::invalid_argument(format!("invalid status: {s}"))),
    }
}

#[tonic::async_trait]
impl IntentService for IntentServer {
    async fn create(
        &self,
        request: Request<CreateIntentRequest>,
    ) -> Result<Response<CreateIntentResponse>, Status> {
        self.security
            .authorize(&request, AuthorizationAction::CreateIntent, None)?;
        let req = request.into_inner();
        let now = now_ms();
        let id = IntentId::new();

        let intent = Intent {
            id,
            title: req.title,
            goal: req.description,
            constraints: vec![],
            acceptance_tests: vec![],
            links: vec![],
            policy_refs: vec![],
            agents: if req.author.is_empty() {
                vec![]
            } else {
                vec![req.author]
            },
            change_ids: vec![],
            depends_on: vec![],
            supersedes: vec![],
            status: IntentStatus::Open,
            created_at_ms: now,
            updated_at_ms: now,
        };

        let store = self.store.write().await;
        let obj_id = store
            .store_object(&Object::Intent(intent.clone()))
            .map_err(|e| Status::internal(e.to_string()))?;
        store
            .set_ref(&format!("intents/{id}"), &obj_id)
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(CreateIntentResponse {
            intent: Some(intent_to_proto(&intent)),
        }))
    }

    async fn get(
        &self,
        request: Request<GetIntentRequest>,
    ) -> Result<Response<GetIntentResponse>, Status> {
        self.security
            .authorize(&request, AuthorizationAction::ReadIntent, None)?;
        let req = request.into_inner();
        let ulid = req
            .id
            .ok_or_else(|| Status::invalid_argument("missing id"))?;
        let arr: [u8; 16] = ulid
            .data
            .as_slice()
            .try_into()
            .map_err(|_| Status::invalid_argument("invalid ULID"))?;
        let intent_id = IntentId::from_bytes(arr);

        let store = self.store.read().await;
        let obj_id = store
            .get_ref(&format!("intents/{intent_id}"))
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("intent not found"))?;

        match store
            .load_object(&obj_id)
            .map_err(|e| Status::internal(e.to_string()))?
        {
            Object::Intent(i) => Ok(Response::new(GetIntentResponse {
                intent: Some(intent_to_proto(&i)),
            })),
            _ => Err(Status::internal("object is not an intent")),
        }
    }

    async fn list(
        &self,
        request: Request<ListIntentsRequest>,
    ) -> Result<Response<ListIntentsResponse>, Status> {
        self.security
            .authorize(&request, AuthorizationAction::ReadIntent, None)?;
        let req = request.into_inner();
        let store = self.store.read().await;
        let refs = store
            .list_refs("intents")
            .map_err(|e| Status::internal(e.to_string()))?;

        let mut intents = Vec::new();
        for (_, obj_id) in &refs {
            if let Ok(Object::Intent(i)) = store.load_object(obj_id) {
                if req.status_filter.is_empty() || status_matches(&i.status, &req.status_filter) {
                    intents.push(intent_to_proto(&i));
                }
            }
        }

        Ok(Response::new(ListIntentsResponse { intents }))
    }

    async fn update(
        &self,
        request: Request<UpdateIntentRequest>,
    ) -> Result<Response<UpdateIntentResponse>, Status> {
        self.security
            .authorize(&request, AuthorizationAction::UpdateIntent, None)?;
        let req = request.into_inner();
        let ulid = req
            .id
            .ok_or_else(|| Status::invalid_argument("missing id"))?;
        let arr: [u8; 16] = ulid
            .data
            .as_slice()
            .try_into()
            .map_err(|_| Status::invalid_argument("invalid ULID"))?;
        let intent_id = IntentId::from_bytes(arr);

        let store = self.store.write().await;
        let obj_id = store
            .get_ref(&format!("intents/{intent_id}"))
            .map_err(|e| Status::internal(e.to_string()))?
            .ok_or_else(|| Status::not_found("intent not found"))?;

        let mut intent = match store
            .load_object(&obj_id)
            .map_err(|e| Status::internal(e.to_string()))?
        {
            Object::Intent(i) => i,
            _ => return Err(Status::internal("not an intent")),
        };

        if !req.title.is_empty() {
            intent.title = req.title;
        }
        if !req.description.is_empty() {
            intent.goal = req.description;
        }
        if !req.status.is_empty() {
            intent.status = parse_status(&req.status)?;
        }
        intent.updated_at_ms = now_ms();

        let new_obj_id = store
            .store_object(&Object::Intent(intent.clone()))
            .map_err(|e| Status::internal(e.to_string()))?;
        store
            .set_ref(&format!("intents/{intent_id}"), &new_obj_id)
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(UpdateIntentResponse {
            intent: Some(intent_to_proto(&intent)),
        }))
    }
}

fn status_matches(s: &IntentStatus, filter: &str) -> bool {
    matches!(
        (s, filter),
        (IntentStatus::Open, "open")
            | (IntentStatus::Blocked, "blocked")
            | (IntentStatus::Done, "done")
            | (IntentStatus::Superseded, "superseded")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::{AuthorizationRole, RoleBasedAuthorizer, PRINCIPAL_METADATA_KEY};

    #[tokio::test]
    async fn reader_role_cannot_create_intent() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Arc::new(RwLock::new(ClawStore::init(tmp.path()).unwrap()));
        let server = IntentServer::new(store).with_authorizer(Arc::new(
            RoleBasedAuthorizer::new().grant_role("reader", AuthorizationRole::Reader),
        ));
        let mut request = Request::new(CreateIntentRequest {
            title: "intent".to_string(),
            description: "goal".to_string(),
            author: "reader".to_string(),
            labels: vec![],
        });
        request
            .metadata_mut()
            .insert(PRINCIPAL_METADATA_KEY, "reader".parse().unwrap());

        let err = server.create(request).await.unwrap_err();

        assert_eq!(err.code(), tonic::Code::PermissionDenied);
        assert!(err.message().contains("missing required scope"));
    }
}
