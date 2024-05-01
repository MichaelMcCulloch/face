use faiss::error::Error as FsError;

use std::{
    error::Error as StdError,
    fmt::{Debug, Display, Formatter, Result},
};

#[derive(Debug)]
pub enum IndexError {
    FileNotFound,
    IndexError(FsError),
}

#[derive(Debug)]
pub enum IndexSearchError {
    IndexSearchError(FsError),
}

impl StdError for IndexError {}
impl StdError for IndexSearchError {}

impl Display for IndexError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            IndexError::FileNotFound => write!(f, "SearchService: Index not found"),
            IndexError::IndexError(err) => {
                write!(f, "SearchService: {}", err)
            }
        }
    }
}

impl Display for IndexSearchError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            IndexSearchError::IndexSearchError(err) => {
                write!(f, "SearchService: {}", err)
            }
        }
    }
}

impl From<FsError> for IndexError {
    fn from(value: FsError) -> Self {
        Self::IndexError(value)
    }
}
impl From<FsError> for IndexSearchError {
    fn from(value: FsError) -> Self {
        Self::IndexSearchError(value)
    }
}
