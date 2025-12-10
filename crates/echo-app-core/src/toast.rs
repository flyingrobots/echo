// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Simple toast queue with TTL + dedupe for Echo tools.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Toast severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    /// Informational note.
    Info,
    /// Warning that may need attention.
    Warn,
    /// Error requiring user awareness.
    Error,
}

/// Scope of a toast (who should see it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastScope {
    /// Visible to all tools/sessions.
    Global,
    /// Scoped to a specific session/RMG.
    Session,
    /// Local to the current tool only.
    Local,
}

/// Identifier for a toast entry.
pub type ToastId = u64;

/// Toast data stored in the service.
#[derive(Debug, Clone)]
pub struct Toast {
    /// Stable identifier.
    pub id: ToastId,
    /// Severity.
    pub kind: ToastKind,
    /// Scope.
    pub scope: ToastScope,
    /// Short title line.
    pub title: String,
    /// Optional body text.
    pub body: Option<String>,
    /// Time-to-live duration.
    pub ttl: Duration,
    /// Creation time.
    pub created: Instant,
}

/// Rendering-friendly view of a toast.
#[derive(Debug, Clone)]
pub struct ToastRender {
    /// Stable identifier.
    pub id: ToastId,
    /// Severity.
    pub kind: ToastKind,
    /// Scope.
    pub scope: ToastScope,
    /// Short title line.
    pub title: String,
    /// Optional body text.
    pub body: Option<String>,
    /// 1.0 -> just created, 0.0 -> expired.
    pub progress: f32,
}

/// In-memory toast queue with TTL and dedupe window.
pub struct ToastService {
    queue: VecDeque<Toast>,
    max: usize,
    dedupe_window: Duration,
    next_id: ToastId,
}

impl ToastService {
    /// Create a new queue with a maximum length.
    pub fn new(max: usize) -> Self {
        Self {
            queue: VecDeque::new(),
            max,
            dedupe_window: Duration::from_millis(500),
            next_id: 1,
        }
    }

    /// Push a toast, deduping identical recent entries (same kind/scope/title/body within `dedupe_window`).
    pub fn push<S, B>(
        &mut self,
        kind: ToastKind,
        scope: ToastScope,
        title: S,
        body: B,
        ttl: Duration,
        now: Instant,
    ) -> ToastId
    where
        S: Into<String>,
        B: Into<Option<String>>,
    {
        let title = title.into();
        let body_opt = body.into();

        if let Some(existing) = self.queue.iter_mut().find(|t| {
            t.kind == kind
                && t.scope == scope
                && t.title == title
                && t.body == body_opt
                && now.duration_since(t.created) <= self.dedupe_window
        }) {
            existing.created = now;
            existing.ttl = ttl;
            return existing.id;
        }

        let id = self.next_id;
        self.next_id += 1;
        let toast = Toast {
            id,
            kind,
            scope,
            title,
            body: body_opt,
            ttl,
            created: now,
        };
        if self.queue.len() == self.max {
            self.queue.pop_front();
        }
        self.queue.push_back(toast);
        id
    }

    /// Drop expired toasts (call once per frame/tick).
    pub fn retain_visible(&mut self, now: Instant) {
        self.queue.retain(|t| now.duration_since(t.created) < t.ttl);
    }

    /// Return render-ready toasts with progress ratios.
    pub fn visible(&self, now: Instant) -> Vec<ToastRender> {
        self.queue
            .iter()
            .filter(|t| now.duration_since(t.created) < t.ttl)
            .map(|t| ToastRender {
                id: t.id,
                kind: t.kind,
                scope: t.scope,
                title: t.title.clone(),
                body: t.body.clone(),
                progress: 1.0 - (now.duration_since(t.created).as_secs_f32() / t.ttl.as_secs_f32()),
            })
            .collect()
    }
}
