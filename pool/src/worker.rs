use super::PoolError;
use std::thread;
use crossbeam::channel;
use std::panic;
use std::panic::UnwindSafe;
use core::marker::PhantomData;

/// Message sent to worker
pub enum WorkerMessage<T> {
    Work(T),
    Resign
}

#[derive(Debug, PartialOrd, PartialEq)]
pub enum WorkerResult {
    Panic,
    Ok
}

/// Worker manages a thread that does work
pub struct Worker<T>
    where
        T: FnOnce() + Send + 'static + UnwindSafe,
{
    join_handle: thread::JoinHandle<WorkerResult>,
    _t: PhantomData<T>,
}

impl<T: FnOnce() + Send + 'static + UnwindSafe> Worker<T> {
    pub fn spawn(receiver: channel::Receiver<WorkerMessage<T>>) -> Self {
        let join_handle = thread::spawn(move || {

            let mut panic_occurred = false;

            'msg_loop: while let Some(message) = receiver.recv() {
                match message {
                    WorkerMessage::Work(work) => {
                        let result = panic::catch_unwind(|| work());

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
        }
    }

    pub fn join(self) -> Result<WorkerResult, PoolError> {
        self.join_handle.join().map_err(|err| PoolError::CouldNotJoin(format!("{:?}", err)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam::channel;
    #[test]
    fn worker_lifetime() {
        let (s, r) = channel::unbounded();
        let worker= Worker::<fn()>::spawn(r);

        s.send(WorkerMessage::Resign);
        worker.join().unwrap();
    }

    #[test]
    fn worker_work() {
        let (s, r) = channel::unbounded();
        let worker= Worker::<fn()>::spawn(r);

        s.send(WorkerMessage::Work(|| panic!("This should panic!")));

        s.send(WorkerMessage::Resign);
        assert_eq!(worker.join().unwrap(), WorkerResult::Panic);
    }
}

