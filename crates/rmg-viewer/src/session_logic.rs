// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Pure-ish session frame handling: apply snapshots/diffs and raise toasts.

use std::time::Instant;

use echo_app_core::toast::{ToastKind, ToastScope, ToastService};
use echo_graph::RmgFrame;

use crate::{core::{Screen, UiState}, scene::scene_from_wire, viewer_state::ViewerState};

pub struct FrameOutcome {
    pub desync: Option<String>,
    pub enter_view: bool,
}

/// Apply a batch of RMG frames; returns a desync reason if we should drop the connection.
pub(crate) fn process_frames(
    ui: &mut UiState,
    viewer: &mut ViewerState,
    toasts: &mut ToastService,
    frames: impl IntoIterator<Item = RmgFrame>,
) -> FrameOutcome {
    let mut outcome = FrameOutcome {
        desync: None,
        enter_view: false,
    };
    for frame in frames {
        match frame {
            RmgFrame::Snapshot(s) => {
                viewer.wire_graph = s.graph;
                viewer.epoch = Some(s.epoch);
                viewer.history.append(viewer.wire_graph.clone(), s.epoch);
                viewer.graph = scene_from_wire(&viewer.wire_graph);
                ui.screen = Screen::View;
                outcome.enter_view = true;
                if let Some(expected) = s.state_hash {
                    let actual = viewer.wire_graph.compute_hash();
                    if actual != expected {
                        toasts.push(
                            ToastKind::Error,
                            ToastScope::Local,
                            "Snapshot hash mismatch",
                            None,
                            std::time::Duration::from_secs(6),
                            Instant::now(),
                        );
                    }
                }
            }
            RmgFrame::Diff(d) => {
                let Some(epoch) = viewer.epoch else {
                    toasts.push(
                        ToastKind::Error,
                        ToastScope::Local,
                        "Diff received before snapshot",
                        None,
                        std::time::Duration::from_secs(6),
                        Instant::now(),
                    );
                    continue;
                };
                if d.from_epoch != epoch || d.to_epoch != epoch + 1 {
                    toasts.push(
                        ToastKind::Error,
                        ToastScope::Local,
                        "Protocol violation: non-sequential diff",
                        Some(format!(
                            "from={}, to={}, local={}",
                            d.from_epoch, d.to_epoch, epoch
                        )),
                        std::time::Duration::from_secs(8),
                        Instant::now(),
                    );
                    outcome.desync = Some("Desynced (gap) — reconnect".into());
                    return outcome;
                }
                for op in d.ops {
                    if let Err(err) = viewer.wire_graph.apply_op(op) {
                        toasts.push(
                            ToastKind::Error,
                            ToastScope::Local,
                            "Failed applying RMG op",
                            Some(format!("{err:#}")),
                            std::time::Duration::from_secs(8),
                            Instant::now(),
                        );
                        outcome.desync = Some("Desynced (apply failed) — reconnect".into());
                        return outcome;
                    }
                }
                viewer.epoch = Some(d.to_epoch);
                if let Some(expected) = d.state_hash {
                    let actual = viewer.wire_graph.compute_hash();
                    if actual != expected {
                        toasts.push(
                            ToastKind::Error,
                            ToastScope::Local,
                            "State hash mismatch",
                            Some(format!("expected {:?}, got {:?}", expected, actual)),
                            std::time::Duration::from_secs(8),
                            Instant::now(),
                        );
                        outcome.desync = Some("Desynced (hash mismatch) — reconnect".into());
                        return outcome;
                    }
                }
                viewer.history.append(viewer.wire_graph.clone(), d.to_epoch);
                viewer.graph = scene_from_wire(&viewer.wire_graph);
                ui.screen = Screen::View;
                outcome.enter_view = true;
            }
        }
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    use echo_graph::{RenderGraph, RmgDiff, RmgFrame, RmgSnapshot};

    #[test]
    fn snapshot_enters_view() {
        let mut ui = UiState::new();
        let mut viewer = ViewerState::default();
        let mut toasts = ToastService::new(8);
        let snap = RmgFrame::Snapshot(RmgSnapshot {
            epoch: 0,
            graph: RenderGraph::default(),
            state_hash: None,
        });
        let outcome = process_frames(&mut ui, &mut viewer, &mut toasts, [snap]);
        assert!(outcome.enter_view);
        assert!(outcome.desync.is_none());
        assert!(matches!(ui.screen, crate::Screen::View));
    }

    #[test]
    fn gap_diff_desyncs() {
        let mut ui = UiState::new();
        let mut viewer = ViewerState::default();
        let mut toasts = ToastService::new(8);
        // first set epoch via snapshot
        let snap = RmgFrame::Snapshot(RmgSnapshot {
            epoch: 0,
            graph: RenderGraph::default(),
            state_hash: None,
        });
        let _ = process_frames(&mut ui, &mut viewer, &mut toasts, [snap]);
        // gap diff
        let diff = RmgFrame::Diff(RmgDiff {
            from_epoch: 2,
            to_epoch: 3,
            ops: vec![],
            state_hash: None,
        });
        let outcome = process_frames(&mut ui, &mut viewer, &mut toasts, [diff]);
        assert!(outcome.desync.is_some());
    }
}
