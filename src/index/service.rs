use std::error::Error;


pub(super) struct Query<const N: usize> {
    pub(super) embedding: [f32; N],
    pub(super) neighbors: usize,
}
pub(crate) trait SearchService<const N: usize> {
    type E: Error;
    fn search(&mut self, query: Query<N>) -> Result<Vec<i64>, Self::E>;
    fn search_batch(
        &mut self,
        query: &[[f32; N]],
        neighbors: usize,
    ) -> Result<Vec<Vec<i64>>, Self::E>;
}
