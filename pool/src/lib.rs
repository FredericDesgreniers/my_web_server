use core::marker::PhantomData;
use crossbeam as channel;
use std::thread;

#[derive(Debug)]
pub enum PoolError {
    CouldNotJoin,
}

/// Takes care of sending work to worker threads
pub struct ThreadPool<T>
where
    T: FnOnce() + Send + 'static,
{
    workers: Vec<Worker<T>>,
    sender: channel::Sender<T>,
}

impl<T: FnOnce() + Send + 'static> ThreadPool<T> {
    /// Create a new ThreadPool
    /// `worker_num` number of worker threads created
    pub fn new(worker_num: usize) -> Self {
        let mut workers = Vec::with_capacity(worker_num);

        let (sender, receiver) = channel::unbounded();

        for _ in 0..worker_num {
            workers.push(Worker::spawn(receiver.clone()));
        }

        Self { workers, sender }
    }

    /// Send work to a worker thread
    pub fn do_work(&self, work: T) {
        self.sender.send(work);
    }

    pub fn join(self) -> Result<(), PoolError> {
        for worker in self.workers {
            worker
                .join_handle
                .join()
                .map_err(|_| PoolError::CouldNotJoin)?;
        }

        Ok(())
    }
}

/// Worker manages a thread that does work
pub struct Worker<T>
where
    T: FnOnce() + Send + 'static,
{
    join_handle: thread::JoinHandle<()>,
    _t: PhantomData<T>,
}

impl<T: FnOnce() + Send + 'static> Worker<T> {
    pub fn spawn(receiver: channel::Receiver<T>) -> Self {
        let join_handle = thread::spawn(move || {
            while let Some(work) = receiver.recv() {
                work()
            }
        });

        Self {
            join_handle,
            _t: PhantomData::default(),
        }
    }
}
