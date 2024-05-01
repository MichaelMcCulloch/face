use faiss::{
    index::{pretransform::PreTransformIndexImpl, IndexImpl},
    Index,
};
use std::{
    path::{Path, PathBuf},
    time::Instant,
};

use super::{IndexError, IndexSearchError, SearchService};

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

impl SearchService for FaissIndex {
    type E = IndexSearchError;

    fn search(&mut self, query: &[f32], neighbors: usize) -> Result<Vec<i64>, Self::E> {
        let start = Instant::now();
        let rs = self.index.search(query, neighbors)?;
        let indices: Vec<i64> = rs.labels.iter().map(|i| i.to_native()).collect();
        log::debug!("Index {:?}", start.elapsed());
        Ok(indices)
    }
}
