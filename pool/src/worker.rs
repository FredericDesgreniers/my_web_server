use super::PoolError;
use core::marker::PhantomData;
use crossbeam::channel;
use std::panic;
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::thread;

/// Message sent to worker
pub enum WorkerMessage<T> {
    Work(T),
    Resign,
}

#[derive(Debug, PartialOrd, PartialEq)]
pub enum WorkerResult {
    Panic,
    Ok,
}

/// Worker manages a thread that does work
pub struct Worker<S, T>
where
    T: FnOnce(&S) + Send + 'static + UnwindSafe,
    S: Send + Sync + RefUnwindSafe + 'static,
{
    join_handle: thread::JoinHandle<WorkerResult>,
    _t: PhantomData<T>,
    _s: PhantomData<S>,
}

impl<S, T> Worker<S, T>
where
    S: Send + Sync + RefUnwindSafe + 'static,
    T: FnOnce(&S) + Send + 'static + UnwindSafe,
{
    pub fn spawn(receiver: channel::Receiver<WorkerMessage<T>>, state: S) -> Self {
        let join_handle = thread::spawn(move || {
            let mut panic_occurred = false;

            'msg_loop: while let Some(message) = receiver.recv() {
                match message {
                    WorkerMessage::Work(work) => {
                        let result = panic::catch_unwind(|| work(&state));

                        if result.is_err() {
                            panic_occurred = true;
                        }
                    }
                    WorkerMessage::Resign => {
                        break 'msg_loop;
                    }
                }
            }

            if panic_occurred {
                WorkerResult::Panic
            } else {
                WorkerResult::Ok
            }
        });

        Self {
            join_handle,
            _t: PhantomData::default(),
            _s: PhantomData::default(),
        }
    }

    pub fn join(self) -> Result<WorkerResult, PoolError> {
        self.join_handle
            .join()
            .map_err(|err| PoolError::CouldNotJoin(format!("{:?}", err)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam::channel;
    #[test]
    fn worker_lifetime() {
        let (s, r) = channel::unbounded();
        let worker = Worker::<fn()>::spawn(r);

        s.send(WorkerMessage::Resign);
        worker.join().unwrap();
    }

    #[test]
    fn worker_work() {
        let (s, r) = channel::unbounded();
        let worker = Worker::<fn()>::spawn(r);

        s.send(WorkerMessage::Work(|| panic!("This should panic!")));

        s.send(WorkerMessage::Resign);
        assert_eq!(worker.join().unwrap(), WorkerResult::Panic);
    }
}
