use faiss::{
    index::{pretransform::PreTransformIndexImpl, IndexImpl},
    Index,
};
use std::{
    path::{Path, PathBuf},
    time::Instant,
};

use super::{service::Query, IndexError, IndexSearchError, SearchService};

pub(crate) struct FaissIndex {
    index: PreTransformIndexImpl<IndexImpl>,
}

impl FaissIndex {
    pub(crate) fn new<P: AsRef<Path>>(index_path: &P) -> Result<Self, IndexError> {
        let start = Instant::now();

        let index_path = PathBuf::from(index_path.as_ref());
        if !index_path.exists() {
            return Err(IndexError::FileNotFound);
        }

        let index = faiss::read_index(index_path.to_str().expect("Index path is not a string"))?
            .into_pre_transform()?;

        log::info!("Load Index {:?}", start.elapsed());
        Ok(FaissIndex { index })
    }
}

impl<const N: usize> SearchService<N> for FaissIndex {
    type E = IndexSearchError;

    fn search(&mut self, query: Query<N>) -> Result<Vec<i64>, Self::E> {
        let start = Instant::now();
        let rs = self.index.search(&query.embedding, query.neighbors)?;
        let indices: Vec<i64> = rs.labels.iter().map(|i| i.to_native()).collect();
        log::debug!("Index {:?}", start.elapsed());
        Ok(indices)
    }

    fn search_batch(
        &mut self,
        query: &[[f32; N]],
        neighbors: usize,
    ) -> Result<Vec<Vec<i64>>, Self::E> {
        let start = Instant::now();

        let input_to_faiss = query.iter().fold(vec![], |mut v, q| {
            v.extend(q.iter());
            v
        });

        let rs = self.index.search(&input_to_faiss, neighbors)?;
        let indices: Vec<i64> = rs.labels.iter().map(|i| i.to_native()).collect();

        let output_from_faiss = indices
            .chunks_exact(neighbors)
            .map(|e| e.to_vec())
            .collect::<Vec<_>>();
        log::debug!("Index {:?}", start.elapsed());
        Ok(output_from_faiss)
    }
}
