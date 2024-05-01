use std::error::Error;

pub(crate) trait SearchService<const N: usize> {
    type E: Error;
    fn search(&mut self, query: &[f32; N], neighbors: usize) -> Result<Vec<i64>, Self::E>;
    fn search_batch(
        &mut self,
        query: &[[f32; N]],
        neighbors: usize,
    ) -> Result<Vec<Vec<i64>>, Self::E>;
}
