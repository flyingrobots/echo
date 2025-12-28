// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Tiny rolling frame time tracker.

use std::collections::VecDeque;

#[derive(Clone, Debug)]
pub struct PerfStats {
    frame_ms: VecDeque<f32>,
    max_samples: usize,
}

impl Default for PerfStats {
    fn default() -> Self {
        Self {
            frame_ms: VecDeque::with_capacity(400),
            max_samples: 400,
        }
    }
}

impl PerfStats {
    pub fn push(&mut self, frame: f32) {
        if self.frame_ms.len() == self.max_samples {
            self.frame_ms.pop_front();
        }
        self.frame_ms.push_back(frame);
    }
    pub fn fps(&self) -> f32 {
        self.frame_ms.back().map(|ms| 1000.0 / ms).unwrap_or(0.0)
    }
}
