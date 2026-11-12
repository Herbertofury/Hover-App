//! A single sleeping worker, one replaceable pending request, and one result slot.
//! Cancellation is cooperative; native decoders still require process isolation.
use crate::decode_job::{CancellationToken, DecodeGeneration, DecodeRequest};
use std::sync::{Arc, Condvar, Mutex};

pub struct WorkerResult<R> {
    pub generation: DecodeGeneration,
    pub value: Result<R, WorkerPanicked>,
}

#[derive(Debug)]
pub struct WorkerPanicked;

struct State<T, R> {
    next_generation: u64,
    pending: Option<DecodeRequest<T>>,
    active: Option<CancellationToken>,
    completed: Option<WorkerResult<R>>,
    shutdown: bool,
}

struct Shared<T, R> {
    state: Mutex<State<T, R>>,
    ready: Condvar,
}

pub struct PreviewWorker<T, R> {
    shared: Arc<Shared<T, R>>,
}

impl<T: Send + 'static, R: Send + 'static> PreviewWorker<T, R> {
    pub fn new(
        decode: impl Fn(T, &CancellationToken) -> R + Send + 'static,
        notify: impl Fn() + Send + 'static,
    ) -> std::io::Result<Self> {
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                next_generation: 1,
                pending: None,
                active: None,
                completed: None,
                shutdown: false,
            }),
            ready: Condvar::new(),
        });
        let thread_shared = Arc::clone(&shared);
        std::thread::Builder::new()
            .name("hover-preview".into())
            .spawn(move || {
                loop {
                    let request = {
                        let mut state = thread_shared.state.lock().unwrap();
                        while state.pending.is_none() && !state.shutdown {
                            state = thread_shared.ready.wait(state).unwrap();
                        }
                        if state.shutdown {
                            return;
                        }
                        let request = state.pending.take().unwrap();
                        state.active = Some(request.token.clone());
                        request
                    };
                    let value = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        decode(request.payload, &request.token)
                    }))
                    .map_err(|_| WorkerPanicked);
                    let publish = {
                        let mut state = thread_shared.state.lock().unwrap();
                        state.active = None;
                        if state.shutdown {
                            return;
                        }
                        if request.token.is_cancelled() {
                            false
                        } else {
                            state.completed = Some(WorkerResult {
                                generation: request.generation,
                                value,
                            });
                            true
                        }
                    };
                    if publish {
                        notify();
                    }
                }
            })?;
        Ok(Self { shared })
    }

    pub fn submit(&self, payload: T) -> DecodeGeneration {
        let mut state = self.shared.state.lock().unwrap();
        cancel_requests(&mut state);
        let generation = DecodeGeneration(state.next_generation);
        state.next_generation = state.next_generation.wrapping_add(1);
        state.pending = Some(DecodeRequest {
            generation,
            token: CancellationToken::new(),
            payload,
        });
        self.shared.ready.notify_one();
        generation
    }

    pub fn cancel(&self) {
        cancel_requests(&mut self.shared.state.lock().unwrap());
    }

    pub fn take_result(&self) -> Option<WorkerResult<R>> {
        self.shared.state.lock().unwrap().completed.take()
    }

    pub fn is_busy(&self) -> bool {
        let state = self.shared.state.lock().unwrap();
        state.active.is_some() || state.pending.is_some()
    }
}

fn cancel_requests<T, R>(state: &mut State<T, R>) {
    if let Some(token) = &state.active {
        token.cancel();
    }
    if let Some(request) = state.pending.take() {
        request.token.cancel();
    }
    state.completed = None;
}

impl<T, R> Drop for PreviewWorker<T, R> {
    fn drop(&mut self) {
        let mut state = self.shared.state.lock().unwrap();
        cancel_requests(&mut state);
        state.shutdown = true;
        self.shared.ready.notify_one();
        // Do not block the UI waiting for an uncooperative native codec.
        // The worker exits after its current decode returns.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn rapid_requests_keep_only_the_latest_and_never_decode_concurrently() {
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let (done_tx, done_rx) = mpsc::channel();
        let worker = PreviewWorker::new(
            move |id, token| {
                started_tx.send(id).unwrap();
                if id == 0 {
                    release_rx.recv().unwrap();
                    assert!(token.is_cancelled());
                }
                id
            },
            move || done_tx.send(()).unwrap(),
        )
        .unwrap();
        worker.submit(0);
        assert_eq!(started_rx.recv_timeout(Duration::from_secs(5)).unwrap(), 0);
        for id in 1..=10_000 {
            worker.submit(id);
        }
        assert!(started_rx.try_recv().is_err());
        release_tx.send(()).unwrap();
        assert_eq!(
            started_rx.recv_timeout(Duration::from_secs(5)).unwrap(),
            10_000
        );
        done_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(worker.take_result().unwrap().value.unwrap(), 10_000);
        assert!(worker.take_result().is_none());
        assert!(!worker.is_busy());
    }

    #[test]
    fn cancel_discards_late_result_and_worker_recovers_from_panic() {
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let (done_tx, done_rx) = mpsc::channel();
        let worker = PreviewWorker::new(
            move |id, token| {
                if id == 0 {
                    started_tx.send(token.clone()).unwrap();
                    release_rx.recv().unwrap();
                }
                assert_ne!(id, 1, "malformed decoder regression");
                id
            },
            move || done_tx.send(()).unwrap(),
        )
        .unwrap();
        worker.submit(0);
        let token = started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        worker.cancel();
        assert!(token.is_cancelled());
        release_tx.send(()).unwrap();
        worker.submit(1);
        done_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(worker.take_result().unwrap().value.is_err());
        let generation = worker.submit(2);
        done_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let result = worker.take_result().unwrap();
        assert_eq!(result.generation, generation);
        assert_eq!(result.value.unwrap(), 2);
    }
}
