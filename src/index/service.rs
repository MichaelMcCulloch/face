use std::error::Error;

pub(crate) trait SearchService {
    type E: Error;
    fn search(&mut self, query: &Vec<f32>, neighbors: usize) -> Result<Vec<i64>, Self::E>;
}
