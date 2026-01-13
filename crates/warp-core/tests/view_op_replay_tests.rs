// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>

#![allow(missing_docs)]
use ciborium::value::Value as CborValue;
use echo_wasm_abi::encode_cbor;
use warp_core::{
    cmd, make_node_id, make_type_id, AtomPayload, AttachmentValue, GraphStore, NodeRecord,
};

#[test]
fn test_view_op_replay_consistency() {
    let mut store = GraphStore::default();

    // --- 1. Test RoutePush (Unwrapped) ---
    let route_vars = CborValue::Text("/dashboard".to_string());
    let route_payload = CborValue::Map(vec![(
        CborValue::Text("vars".to_string()),
        route_vars.clone(),
    )]);
    let route_payload_bytes = encode_cbor(&route_payload).unwrap();

    let event_route = make_node_id("event-route");
    store.insert_node(
        event_route,
        NodeRecord {
            ty: make_type_id("sim/inbox/event"),
        },
    );
    store.set_node_attachment(
        event_route,
        Some(AttachmentValue::Atom(AtomPayload::new(
            make_type_id("intent:route_push"),
            route_payload_bytes.into(),
        ))),
    );

    cmd::route_inbox_event(&mut store, &event_route);

    // --- 2. Test SetTheme (Unwrapped) ---
    let theme_vars = CborValue::Text("dark".to_string());
    let theme_payload = CborValue::Map(vec![(
        CborValue::Text("vars".to_string()),
        theme_vars.clone(),
    )]);
    let theme_payload_bytes = encode_cbor(&theme_payload).unwrap();

    let event_theme = make_node_id("event-theme");
    store.insert_node(
        event_theme,
        NodeRecord {
            ty: make_type_id("sim/inbox/event"),
        },
    );
    store.set_node_attachment(
        event_theme,
        Some(AttachmentValue::Atom(AtomPayload::new(
            make_type_id("intent:set_theme"),
            theme_payload_bytes.into(),
        ))),
    );

    cmd::route_inbox_event(&mut store, &event_theme);

    // --- 3. Test ToggleNav (Wrapped) ---
    let event_nav = make_node_id("event-nav");
    store.insert_node(
        event_nav,
        NodeRecord {
            ty: make_type_id("sim/inbox/event"),
        },
    );
    store.set_node_attachment(
        event_nav,
        Some(AttachmentValue::Atom(AtomPayload::new(
            make_type_id("intent:toggle_nav"),
            vec![].into(), // payload doesn't matter for toggle
        ))),
    );

    cmd::route_inbox_event(&mut store, &event_nav);

    // Verify live ops
    // Seq 0: RoutePush
    // Seq 1: SetTheme
    // Seq 2: ToggleNav

    let live_route_payload = get_view_op_payload(&store, 0);
    let live_theme_payload = get_view_op_payload(&store, 1);
    let live_nav_payload = get_view_op_payload(&store, 2);

    // Expected: RoutePush and SetTheme should be just the "vars" value, not the whole map.
    assert_eq!(live_route_payload, encode_cbor(&route_vars).unwrap());
    assert_eq!(live_theme_payload, encode_cbor(&theme_vars).unwrap());

    // Expected: ToggleNav should be {"open": true}
    let expected_nav = CborValue::Map(vec![(
        CborValue::Text("open".to_string()),
        CborValue::Bool(true),
    )]);
    assert_eq!(live_nav_payload, encode_cbor(&expected_nav).unwrap());

    // --- Run Replay ---
    cmd::project_state(&mut store);

    // Replay emits: Theme, Nav, Route
    // Seq 3: SetTheme (from project_state)
    // Seq 4: ToggleNav (from project_state)
    // Seq 5: RoutePush (from project_state)

    let replay_theme_payload = get_view_op_payload(&store, 3);
    let replay_nav_payload = get_view_op_payload(&store, 4);
    let replay_route_payload = get_view_op_payload(&store, 5);

    // CURRENTLY THESE FAIL (this is the reproduction)
    assert_eq!(
        replay_theme_payload, live_theme_payload,
        "Theme payload mismatch in replay"
    );
    assert_eq!(
        replay_nav_payload, live_nav_payload,
        "Nav payload mismatch in replay"
    );
    assert_eq!(
        replay_route_payload, live_route_payload,
        "Route payload mismatch in replay"
    );
}

fn get_view_op_payload(store: &GraphStore, seq: u64) -> Vec<u8> {
    let op_id = make_node_id(&format!("sim/view/op:{seq:016}"));
    let Some(AttachmentValue::Atom(atom)) = store.node_attachment(&op_id) else {
        panic!("Missing view op at seq {}", seq);
    };
    atom.bytes.to_vec()
}
