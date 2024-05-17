use std::ptr::NonNull;

pub struct Block {
  pub size: usize,
  pub offset: usize,
  pub next_free: Option<NonNull<Block>>,
  pub prev_free: Option<NonNull<Block>>,
  pub next_physical: Option<NonNull<Block>>,
  pub prev_physical: Option<NonNull<Block>>,
}

pub struct BlockMap {
  pub bin_idx: usize,
  pub sub_bin_idx: usize,
  pub rounded_size: usize,
  pub idx: usize,
}
impl Block {
  pub fn is_free(&self) -> bool {
    self.prev_free.is_none()
  }

  pub fn mark_used(&mut self, adjustment: usize) {
    self.prev_free = Some(NonNull::new(self).unwrap());
    self.next_free = Some(NonNull::new(self as *mut _).unwrap());
    self.offset += adjustment;
  }

  pub fn mark_free(&mut self) {
    self.prev_free = None;
    self.next_free = None;
  }
}
