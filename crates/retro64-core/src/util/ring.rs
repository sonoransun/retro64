//! Simple audio ring buffer (host-managed, single producer / single consumer).

use std::collections::VecDeque;

/// Thin wrapper around VecDeque with `push_many`/`pop_many`.
pub struct AudioRing {
    inner: VecDeque<i16>,
    cap: usize,
}

impl AudioRing {
    /// New ring with a soft cap (excess samples are dropped oldest-first).
    pub fn with_capacity(cap: usize) -> Self {
        AudioRing { inner: VecDeque::with_capacity(cap), cap }
    }

    /// Push a batch of samples, dropping oldest if the ring overflows.
    pub fn push_many(&mut self, samples: &[i16]) {
        for s in samples {
            if self.inner.len() == self.cap {
                self.inner.pop_front();
            }
            self.inner.push_back(*s);
        }
    }

    /// Pop up to `n` samples into `out`, returning how many were written.
    pub fn pop_many(&mut self, out: &mut [i16]) -> usize {
        let n = out.len().min(self.inner.len());
        for i in 0..n {
            out[i] = self.inner.pop_front().unwrap();
        }
        n
    }

    /// Current samples buffered.
    pub fn len(&self) -> usize { self.inner.len() }
    /// Empty?
    pub fn is_empty(&self) -> bool { self.inner.is_empty() }
}
