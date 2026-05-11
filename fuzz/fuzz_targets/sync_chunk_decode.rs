#![no_main]

use claw_sync::proto::sync::{FetchObjectsRequest, ObjectChunk, PushObjectsResponse};
use libfuzzer_sys::fuzz_target;
use prost::Message;

fuzz_target!(|data: &[u8]| {
    let _ = ObjectChunk::decode(data);
    let _ = FetchObjectsRequest::decode(data);
    let _ = PushObjectsResponse::decode(data);
});
