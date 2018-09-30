mod worker;

#[macro_use]
extern crate failure;
extern crate core;

use self::worker::{Worker, WorkerMessage, WorkerResult};
use crossbeam as channel;
use std::panic::{RefUnwindSafe, UnwindSafe};

#[derive(Debug, Fail)]
pub enum PoolError {
    #[fail(display = "Could not join: {}", 0)]
    CouldNotJoin(String),
}

/// Takes care of sending work to worker threads
pub struct ThreadPool<S, T>
where
    T: FnOnce(&S) + Send + 'static + UnwindSafe,
    S: RefUnwindSafe + UnwindSafe + Send + Sync + 'static,
{
    workers: Vec<Worker<S, T>>,
    sender: channel::Sender<WorkerMessage<T>>,
}

impl<S, T> ThreadPool<S, T>
where
    S: Clone + RefUnwindSafe + UnwindSafe + Send + Sync + 'static,
    T: FnOnce(&S) + Send + 'static + UnwindSafe,
{
    /// Create a new ThreadPool
    /// `worker_num` number of worker threads created
    pub fn new(worker_num: usize, state: S) -> Self {
        let mut workers = Vec::with_capacity(worker_num);

        let (sender, receiver) = channel::unbounded();

        for _ in 0..worker_num {
            workers.push(Worker::spawn(receiver.clone(), state.clone()));
        }

        Self { workers, sender }
    }

    /// Send work to a worker thread
    pub fn do_work(&self, work: T) {
        self.sender.send(WorkerMessage::Work(work));
    }

    pub fn join(self) -> Result<Vec<WorkerResult>, PoolError> {
        for _ in 0..self.workers.len() {
            self.sender.send(WorkerMessage::Resign);
        }

        self.workers
            .into_iter()
            .map(|worker| worker.join())
            .collect::<Result<Vec<WorkerResult>, PoolError>>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifetime_test() {
        let pool = ThreadPool::<fn()>::new(3);
        pool.join().unwrap();
    }

    #[test]
    fn work_test() {
        let pool = ThreadPool::new(1);

        pool.do_work(move || {
            panic!("This should panic!");
        });

        let result = pool.join().unwrap();

        assert_eq!(result[0], WorkerResult::Panic);
    }
}
