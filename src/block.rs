pub struct BlockHeader {
  pub size: usize,
  pub free: bool,
  pub next_free: Option<Box<BlockHeader>>,
  pub prev_free: Option<Box<BlockHeader>>,
}
