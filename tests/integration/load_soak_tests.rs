#[path = "live_daemon_support.rs"]
mod live_daemon_support;

use claw_core::cof::cof_encode;
use claw_core::id::ObjectId;
use claw_core::object::Object;
use claw_core::types::{Blob, Revision};
use claw_store::ClawStore;
use claw_sync::client::SyncClient;
use claw_sync::proto::sync::sync_service_client::SyncServiceClient;
use claw_sync::proto::sync::{HelloRequest, ObjectChunk, RefUpdate, UpdateRefsRequest};
use live_daemon_support::{init_temp_repo, proto_object_id, write_repo_config, LiveDaemon};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Barrier;

fn store_revision(
    store: &ClawStore,
    parent: Option<ObjectId>,
    summary: &str,
    created_at_ms: u64,
) -> ObjectId {
    store
        .store_object(&Object::Revision(Revision {
            change_id: None,
            parents: parent.into_iter().collect(),
            patches: vec![],
            snapshot_base: None,
            tree: None,
            capsule_id: None,
            author: "integration-test".to_string(),
            created_at_ms,
            summary: summary.to_string(),
            policy_evidence: vec![],
        }))
        .expect("store revision")
}

fn small_queue_config() -> &'static str {
    r#"
config_version = 1

[auth]
require_auth_for_daemon = false
default_profile = "default"

[tls]
require_for_non_localhost = true

[timeouts]
io_ms = 5000
git_bridge_ms = 15000
policy_eval_ms = 5000

[retries]
idempotent_only = true
max_attempts = 4
base_backoff_ms = 100
max_backoff_ms = 2000
jitter = true

[queues]
worker_pool_size = 1
queue_capacity = 0
backpressure = true

[telemetry]
structured_logs = true
correlation_ids = true
metrics = true
traces = true

[policy]
fail_closed_integrate = true
fail_closed_ship = true

[backup]
snapshot_interval_min = 60
verify_integrity_on_startup = false
strict_startup_checks = false
"#
}

fn object_chunk_for(store: &ClawStore, id: ObjectId) -> ObjectChunk {
    let object = store.load_object(&id).expect("load object for chunk");
    let payload = object
        .serialize_payload()
        .expect("serialize object payload");
    let cof = cof_encode(object.type_tag(), &payload).expect("encode object chunk");
    ObjectChunk {
        id: Some(proto_object_id(&id)),
        object_type: object.type_tag() as i32,
        data: cof,
        is_last: false,
    }
}

#[tokio::test]
async fn live_push_pressure_rejects_ref_mutation_until_capacity_recovers() {
    let repo = init_temp_repo();
    write_repo_config(repo.path(), small_queue_config());
    let store = ClawStore::open(repo.path()).expect("open remote store");

    let current = store_revision(&store, None, "current", 1);
    let next = store_revision(&store, Some(current), "next", 2);
    store
        .set_ref("heads/main", &current)
        .expect("seed main ref");

    let local_repo = init_temp_repo();
    let local_store = ClawStore::open(local_repo.path()).expect("open local push store");
    let pushed_blob = local_store
        .store_object(&Object::Blob(Blob {
            data: vec![42u8; 64 * 1024],
            media_type: Some("application/octet-stream".to_string()),
        }))
        .expect("store pushed blob");

    let daemon = LiveDaemon::spawn(repo.path(), &[]).await;

    let mut holding_client = SyncServiceClient::connect(daemon.grpc_endpoint.clone())
        .await
        .expect("connect holding client");

    let (push_tx, push_rx) = tokio::sync::mpsc::channel(2);
    push_tx
        .send(object_chunk_for(&local_store, pushed_blob))
        .await
        .expect("enqueue first object chunk");

    let push_task = tokio::spawn(async move {
        holding_client
            .push_objects(tonic::Request::new(
                tokio_stream::wrappers::ReceiverStream::new(push_rx),
            ))
            .await
    });

    let mut saw_backpressure = false;
    for _ in 0..20 {
        let mut probe = SyncServiceClient::connect(daemon.grpc_endpoint.clone())
            .await
            .expect("connect probe client");
        match probe
            .hello(tonic::Request::new(HelloRequest {
                client_version: env!("CARGO_PKG_VERSION").to_string(),
                capabilities: vec!["partial-clone".to_string()],
            }))
            .await
        {
            Err(status) if status.code() == tonic::Code::ResourceExhausted => {
                saw_backpressure = true;
                break;
            }
            Ok(_) => tokio::time::sleep(Duration::from_millis(50)).await,
            Err(status) => {
                panic!("unexpected probe failure while waiting for saturation: {status}")
            }
        }
    }
    assert!(
        saw_backpressure,
        "live push stream never saturated the bounded worker/queue pool"
    );

    let mut mutator = SyncServiceClient::connect(daemon.grpc_endpoint.clone())
        .await
        .expect("connect mutator");
    let overload = mutator
        .update_refs(tonic::Request::new(UpdateRefsRequest {
            updates: vec![RefUpdate {
                name: "heads/main".to_string(),
                old_target: Some(proto_object_id(&current)),
                new_target: Some(proto_object_id(&next)),
                force: false,
            }],
        }))
        .await
        .expect_err("mutation should be rejected while worker is saturated");

    assert_eq!(overload.code(), tonic::Code::ResourceExhausted);
    assert_eq!(
        store
            .get_ref("heads/main")
            .expect("read ref after overload"),
        Some(current)
    );

    push_tx
        .send(ObjectChunk {
            id: None,
            object_type: 0,
            data: vec![],
            is_last: true,
        })
        .await
        .expect("enqueue final chunk");

    let push_response = push_task
        .await
        .expect("join held push task")
        .expect("finish held push request")
        .into_inner();
    assert!(push_response.success, "held push should eventually succeed");

    let mut final_result = None;
    for _ in 0..20 {
        let mut retry = SyncServiceClient::connect(daemon.grpc_endpoint.clone())
            .await
            .expect("connect retry mutator");
        match retry
            .update_refs(tonic::Request::new(UpdateRefsRequest {
                updates: vec![RefUpdate {
                    name: "heads/main".to_string(),
                    old_target: Some(proto_object_id(&current)),
                    new_target: Some(proto_object_id(&next)),
                    force: false,
                }],
            }))
            .await
        {
            Ok(response) => {
                final_result = Some(response.into_inner());
                break;
            }
            Err(status) if status.code() == tonic::Code::ResourceExhausted => {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            Err(status) => panic!("unexpected retry error: {status}"),
        }
    }

    let response = final_result.expect("mutation should succeed after pressure releases");
    assert!(
        response.success,
        "expected successful ref mutation: {}",
        response.message
    );
    assert_eq!(
        store
            .get_ref("heads/main")
            .expect("read ref after recovery"),
        Some(next)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn contended_live_update_refs_keeps_a_single_consistent_winner() {
    let remote_repo = init_temp_repo();
    let remote_store = ClawStore::open(remote_repo.path()).expect("open remote store");
    let base = store_revision(&remote_store, None, "base", 1);
    remote_store
        .set_ref("heads/main", &base)
        .expect("seed remote main ref");

    let local_repo = init_temp_repo();
    let local_store = ClawStore::open(local_repo.path()).expect("open local store");
    let local_base = store_revision(&local_store, None, "base", 1);
    assert_eq!(
        local_base, base,
        "base revision ids must match across stores"
    );

    let candidates: Vec<ObjectId> = (0..12)
        .map(|idx| {
            store_revision(
                &local_store,
                Some(base),
                &format!("candidate-{idx}"),
                (idx + 2) as u64,
            )
        })
        .collect();

    let daemon = LiveDaemon::spawn(remote_repo.path(), &[]).await;
    let mut pusher = SyncClient::connect(&daemon.grpc_endpoint)
        .await
        .expect("connect pusher");
    let push_result = pusher
        .push_objects(&local_store, &candidates)
        .await
        .expect("push candidate revisions");
    assert!(
        push_result.success,
        "push should succeed: {}",
        push_result.message
    );

    let barrier = Arc::new(Barrier::new(candidates.len()));
    let mut tasks = Vec::with_capacity(candidates.len());
    for candidate in candidates.iter().copied() {
        let barrier = barrier.clone();
        let endpoint = daemon.grpc_endpoint.clone();
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            let mut client = SyncClient::connect(&endpoint)
                .await
                .expect("connect contending client");
            client
                .update_refs(&[("heads/main".to_string(), Some(base), candidate)], false)
                .await
                .expect("update_refs should return response")
        }));
    }

    let mut winners = 0usize;
    let mut conflicts = 0usize;
    for task in tasks {
        let response = task.await.expect("join update_refs contender");
        if response.success {
            winners += 1;
        } else {
            conflicts += 1;
            assert!(
                response.message.contains("CAS conflict"),
                "expected CAS conflict, got: {}",
                response.message
            );
        }
    }

    assert_eq!(winners, 1, "exactly one contender should win the ref race");
    assert_eq!(conflicts, candidates.len() - 1);

    let final_ref = remote_store
        .get_ref("heads/main")
        .expect("read final remote ref")
        .expect("main ref should remain set");
    assert!(
        candidates.contains(&final_ref),
        "winning ref target must be one of the pushed candidate revisions"
    );
}
