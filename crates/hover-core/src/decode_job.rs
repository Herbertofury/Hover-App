use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DecodeGeneration(pub u64);

#[derive(Debug, Clone, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

#[derive(Debug, Clone)]
pub struct DecodeRequest<T> {
    pub generation: DecodeGeneration,
    pub token: CancellationToken,
    pub payload: T,
}

#[derive(Debug)]
pub struct ReplacementDecodeSlot<T> {
    next_generation: AtomicU64,
    pending: Mutex<Option<DecodeRequest<T>>>,
    active_token: Mutex<Option<CancellationToken>>,
}

impl<T> Default for ReplacementDecodeSlot<T> {
    fn default() -> Self {
        Self {
            next_generation: AtomicU64::new(1),
            pending: Mutex::new(None),
            active_token: Mutex::new(None),
        }
    }
}

impl<T> ReplacementDecodeSlot<T> {
    pub fn replace(&self, payload: T) -> DecodeGeneration {
        if let Ok(mut active) = self.active_token.lock()
            && let Some(token) = active.take()
        {
            token.cancel();
        }
        if let Ok(mut pending) = self.pending.lock() {
            if let Some(old) = pending.take() {
                old.token.cancel();
            }
            let generation = DecodeGeneration(self.next_generation.fetch_add(1, Ordering::AcqRel));
            let token = CancellationToken::new();
            *pending = Some(DecodeRequest {
                generation,
                token: token.clone(),
                payload,
            });
            generation
        } else {
            DecodeGeneration(0)
        }
    }

    pub fn take_latest(&self) -> Option<DecodeRequest<T>> {
        let request = self.pending.lock().ok()?.take()?;
        if let Ok(mut active) = self.active_token.lock() {
            *active = Some(request.token.clone());
        }
        Some(request)
    }

    pub fn cancel_active(&self) {
        if let Ok(mut active) = self.active_token.lock()
            && let Some(token) = active.take()
        {
            token.cancel();
        }
        if let Ok(mut pending) = self.pending.lock()
            && let Some(request) = pending.take()
        {
            request.token.cancel();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replacement_slot_only_keeps_newest_request() {
        let slot = ReplacementDecodeSlot::default();
        let first = slot.replace("first");
        let second = slot.replace("second");
        assert!(second > first);
        let request = slot.take_latest().unwrap();
        assert_eq!(request.payload, "second");
        assert_eq!(request.generation, second);
    }
}
