use crate::block::BlockHeader;

pub struct TLSF {
  pool: Vec<u8>,
  free_lists: Vec<Vec<BlockHeader>>,
}

impl TLSF {
  fn new(size: usize) -> Self {
    let initial_block = BlockHeader { size, free: true, next_free: None, prev_free: None };
    TLSF { pool: vec![0; size], free_lists: vec![vec![initial_block]] }
  }

  fn malloc(&mut self, size: usize) -> Option<*mut u8> {
    let (fl, sl) = self.mapping(size);
    for list in &mut self.free_lists {
      if let Some(block) = list.iter_mut().find(|block| block.size >= size) {
        block.free = false;
        return Some(self.pool.as_mut_ptr().wrapping_add(block.size));
      }
    }
    None
  }

  fn free(&mut self, ptr: *mut u8) {
    todo!("implement free");
  }

  fn coalesce(&mut self, block: &mut BlockHeader) {
    todo!("implement coalesce");
  }

  fn mapping(&self, size: usize) -> (usize, usize) {
    todo!("implement mapping");
  }
}
