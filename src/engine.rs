use std::fmt::{Display, Formatter};
use std::time::Instant;

use actix_web::rt;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::{mpsc::channel, Mutex};

use crate::index::FaissIndex;

use crate::index::IndexSearchError;
use crate::index::SearchService;

use nolock::queues::{
    mpsc::jiffy::{self, AsyncSender},
    EnqueueError,
};
use tokio::sync::mpsc::Sender;

struct IndexArguments<const N: usize> {
    embedding: [f32; N],
    neighbors: usize,
    sender: Sender<Vec<i64>>,
}

pub(crate) struct IndexEngine<const N: usize> {
    index_queue: AsyncSender<IndexArguments<N>>,
}

#[derive(Debug)]
pub(crate) enum IndexEngineError {
    QueueError(EnqueueError),
    IndexSearchError(IndexSearchError),
    SendError(SendError<Vec<i64>>),
    NoNeighbors,
}

impl std::error::Error for IndexEngineError {}

impl Display for IndexEngineError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexEngineError::QueueError(e) => write!(f, "QueueError {e:?}"),
            IndexEngineError::NoNeighbors => write!(f, "IndexEngine: NoNeighbors"),
            IndexEngineError::IndexSearchError(e) => write!(f, "IndexSearchError {e:?}"),
            IndexEngineError::SendError(e) => write!(f, "SendError {e:?}"),
        }
    }
}

impl<const N: usize> IndexEngine<N> {
    pub(crate) async fn new(index: FaissIndex) -> Self {
        let mutex = Mutex::new(index);
        let parallelism_does_not_kill_machine =
            std::thread::available_parallelism().unwrap().get() / 2;
        let (mut rx, tx) = jiffy::async_queue::<IndexArguments<N>>();
        rt::spawn(async move {
            loop {
                let mut batch_buckets = (0..32).map(|_| vec![]).collect::<Vec<_>>();
                let mut count = 0usize;
                while let Ok(arguments) = rx.try_dequeue() {
                    batch_buckets[arguments.neighbors].push(arguments);
                    count += 1;
                    if count >= parallelism_does_not_kill_machine {
                        break;
                    }
                }

                for (neighbors, mut arguments_set) in batch_buckets
                    .into_iter()
                    .enumerate()
                    .filter(|(_, b)| !b.is_empty())
                {
                    let start = Instant::now();
                    let neighbors = {
                        let batch = &arguments_set
                            .iter()
                            .map(|args| args.embedding)
                            .collect::<Vec<_>>();
                        let batch = batch.as_slice();
                        let faiss_index = &mut mutex.lock().await;

                        faiss_index
                            .search_batch(batch, neighbors)
                            .map_err(IndexEngineError::IndexSearchError)?
                    };
                    log::info!("{}", start.elapsed().as_millis());

                    for neighbor_set in neighbors {
                        let arguments = arguments_set.remove(0);
                        arguments
                            .sender
                            .send(neighbor_set)
                            .await
                            .map_err(IndexEngineError::SendError)?;
                    }
                }
            }
            Ok::<(), IndexEngineError>(())
        });
        Self { index_queue: tx }
    }

    pub(crate) async fn query(
        &self,
        embedding: [f32; N],
        neighbors: usize,
    ) -> Result<Vec<i64>, IndexEngineError> {
        let (sender, mut rx) = channel(1);
        let index_arguments = IndexArguments {
            embedding,
            neighbors,
            sender,
        };
        self.index_queue
            .enqueue(index_arguments)
            .map_err(|(_, e)| IndexEngineError::QueueError(e))?;

        match rx.recv().await {
            Some(neighbors) => Ok(neighbors),
            None => Err(IndexEngineError::NoNeighbors),
        }
    }
}
