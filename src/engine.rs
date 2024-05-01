use crate::index::{FaissIndex, IndexSearchError, SearchService};
use actix_web::rt;
use nolock::queues::{
    mpsc::jiffy::{self, AsyncSender},
    EnqueueError,
};
use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
    sync::Arc,
};
use tokio::sync::{
    mpsc::{channel, error::SendError, Sender},
    oneshot::Receiver as OneShotReceiver,
    Mutex,
};

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
    pub(crate) async fn new(
        index: FaissIndex,
        signal_exit: Arc<Mutex<OneShotReceiver<()>>>,
    ) -> Self {
        let mutex = Mutex::new(index);

        let parallelism_does_not_kill_machine =
            std::thread::available_parallelism().unwrap().get() / 2;

        let (mut rx, tx) = jiffy::async_queue::<IndexArguments<N>>();
        rt::spawn(async move {
            loop {
                if let Ok(()) = signal_exit.lock().await.try_recv() {
                    break;
                }

                let mut batch_buckets = HashMap::new();
                let mut batch_request_count = 0usize;
                while let Ok(arguments) = rx.try_dequeue() {
                    batch_buckets
                        .entry(arguments.neighbors)
                        .or_insert(vec![])
                        .push(arguments);

                    batch_request_count += 1;
                    if batch_request_count >= parallelism_does_not_kill_machine {
                        break;
                    }
                }

                for (neighbors, mut arguments_set) in batch_buckets
                    .into_iter()
                    .filter(|(_, requests)| !requests.is_empty())
                {
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
