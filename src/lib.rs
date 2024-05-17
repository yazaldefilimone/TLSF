// use core::alloc;
use std::alloc::{GlobalAlloc, Layout, System};
use std::ptr::{null_mut, NonNull};

mod block;
use block::Block;
use block::BlockMap;

/// Two-Level Segregated Fit memory allocator
/// https://ricefields.me/2024/04/20/tlsf-allocator.html
/// ================================================================================================

/// Ideally you would store the free-list nodes as header at the start of each memory block.
/// This implement doesn't do so and performs additional memory allocation for the linked list nodes,
/// because its meant to manage GPU device memory.
/// ================================================================================================
///

pub struct SpeedAllocator {
  bins: Vec<Option<NonNull<Block>>>,
  bin_bitmap: u32,
  sub_bin_bitmap: Vec<u32>,
  num_allocation: usize,
  num_free_block: usize,
}

impl SpeedAllocator {
  const LINEAR: u8 = 7;
  const SUB_BIN: u8 = 5;
  const BIN_COUNT: usize = 64 - Self::LINEAR as usize;
  const SUB_BIN_COUNT: usize = 1 << Self::SUB_BIN;
  const MIN_ALLOC_SIZE: usize = 1 << Self::LINEAR;
  const BLOCK_COUNT: usize = (Self::BIN_COUNT - 1) * Self::SUB_BIN_COUNT + 1;

  pub fn new() -> Self {
    Self {
      bins: vec![None; Self::BLOCK_COUNT],
      bin_bitmap: 0,
      sub_bin_bitmap: vec![0; Self::BIN_COUNT],
      num_allocation: 0,
      num_free_block: 0,
    }
  }

  // allocate a new block of memory
  // returns a pointer to the allocated memory
  pub fn allocate(&mut self, size: usize, alignment: usize) -> Option<NonNull<u8>> {
    // verify if the alignment is a power of two and the size is at least the minimum allocation size
    if !is_pow_two(alignment) && size < Self::MIN_ALLOC_SIZE {
      println!(
        "Falha na alocação, size: {}, alignment: {}, min size: {}",
        size,
        alignment,
        Self::MIN_ALLOC_SIZE
      );
      return None;
    }

    let block_map = self.find_free_block(size).ok()?;
    let block = self.bins[block_map.idx].unwrap().as_ptr();
    self.remove_free_block(block, block_map);

    let maybe_split_block = self.use_free_block(block, size, alignment).ok()?;
    if let Some(split_block) = maybe_split_block {
      self.insert_free_block(split_block);
    }
    self.num_allocation += 1;

    Some(unsafe { NonNull::new_unchecked(block as *mut u8) })
  }

  // deallocate a block of memory
  // deallocate the memory pointed to by ptr
  pub fn deallocate(&mut self, ptr: NonNull<u8>) {
    let block = ptr.as_ptr() as *mut Block;
    unsafe {
      (*block).mark_free();
    }
    self.merge_free_block(block);
    self.insert_free_block(block);
    self.num_allocation -= 1;
  }

  // find a free block of memory
  // returns a BlockMap containing the index of the free block
  fn find_free_block(&mut self, size: usize) -> Result<BlockMap, &'static str> {
    let mut map = self.binmap_up(size);
    let mut sub_bin_bitmap = self.sub_bin_bitmap[map.bin_idx] & (!0 << map.sub_bin_idx);

    if sub_bin_bitmap == 0 {
      let bin_bitmap = self.bin_bitmap & (!0 << (map.bin_idx + 1));
      if bin_bitmap == 0 {
        return Err("OutOfFreeBlock");
      }
      map.bin_idx = bin_bitmap.trailing_zeros() as usize;
      sub_bin_bitmap = self.sub_bin_bitmap[map.bin_idx];
    }

    map.sub_bin_idx = sub_bin_bitmap.trailing_zeros() as usize;
    let idx = map.bin_idx * Self::SUB_BIN_COUNT + map.sub_bin_idx;

    Ok(BlockMap { bin_idx: map.bin_idx, sub_bin_idx: map.sub_bin_idx, rounded_size: map.rounded_size, idx })
  }
  // use a free block of memory
  // returns a pointer to the allocated memory
  fn use_free_block(
    &mut self,
    block: *mut Block,
    size: usize,
    alignment: usize,
  ) -> Result<Option<*mut Block>, &'static str> {
    unsafe {
      if !(*block).is_free() {
        return Err("Block is not free");
      }

      let aligned_offset = align_forward((*block).offset, alignment);
      let adjustment = aligned_offset - (*block).offset;
      let size_with_adjustment = size + adjustment;

      if size_with_adjustment > (*block).size {
        return Err("Block size is insufficient");
      }

      let maybe_new_block: Option<*mut Block> = if (*block).size >= size_with_adjustment + Self::MIN_ALLOC_SIZE {
        // if the block is big enough to hold the requested size, split the block
        // and return the new block
        let new_block = System.alloc(Layout::new::<Block>()) as *mut Block;
        if new_block.is_null() {
          return Err("Failed to allocate new block");
        }

        (*new_block).size = (*block).size - size_with_adjustment;
        (*new_block).offset = (*block).offset + size_with_adjustment;

        if let Some(next_physical) = (*block).next_physical {
          (*next_physical.as_ptr()).prev_physical = Some(NonNull::new(new_block).unwrap());
          (*new_block).next_physical = Some(next_physical);
        }

        (*new_block).prev_physical = Some(NonNull::new(block).unwrap());
        (*block).next_physical = Some(NonNull::new(new_block).unwrap());

        Some(new_block)
      } else {
        None
      };

      (*block).offset = aligned_offset;
      (*block).size = size;
      (*block).mark_used(adjustment);

      Ok(maybe_new_block)
    }
  }

  fn insert_free_block(&mut self, block: *mut Block) {
    let map = self.binmap_down(unsafe { (*block).size });
    let idx = map.bin_idx * Self::SUB_BIN_COUNT + map.sub_bin_idx;

    unsafe {
      let current = self.bins[idx];
      (*block).prev_free = None;
      (*block).next_free = current;
      if let Some(mut curr) = current {
        curr.as_mut().prev_free = Some(NonNull::new(block).unwrap());
      }
      self.bins[idx] = Some(NonNull::new(block).unwrap());
    }

    self.bin_bitmap |= 1 << map.bin_idx;
    self.sub_bin_bitmap[map.bin_idx] |= 1 << map.sub_bin_idx;
    self.num_free_block += 1;
  }

  fn remove_free_block(&mut self, block: *mut Block, block_map: BlockMap) {
    unsafe {
      let next = (*block).next_free;
      let prev = (*block).prev_free;

      if let Some(mut n) = next {
        n.as_mut().prev_free = prev;
      }
      if let Some(mut p) = prev {
        p.as_mut().next_free = next;
      }

      if self.bins[block_map.idx] == Some(NonNull::new(block).unwrap()) {
        if next.is_none() {
          self.sub_bin_bitmap[block_map.bin_idx] &= !(1 << block_map.sub_bin_idx);
          if self.sub_bin_bitmap[block_map.bin_idx] == 0 {
            self.bin_bitmap &= !(1 << block_map.bin_idx);
          }
        }
        self.bins[block_map.idx] = next;
      }
    }
    self.num_free_block -= 1;
  }

  fn merge_free_block(&mut self, block: *mut Block) {
    unsafe {
      if let Some(prev_physical) = (*block).prev_physical {
        if prev_physical.as_ref().is_free() {
          self.remove_free_block(prev_physical.as_ptr(), self.binmap_down(prev_physical.as_ref().size));
          (*block).offset = prev_physical.as_ref().offset;
          (*block).size += prev_physical.as_ref().size;
          (*block).prev_physical = prev_physical.as_ref().prev_physical;
          if let Some(mut pre_prev) = (*block).prev_physical {
            pre_prev.as_mut().next_physical = Some(NonNull::new(block).unwrap());
          }
          System.dealloc(prev_physical.as_ptr() as *mut u8, Layout::new::<Block>());
        }
      }

      if let Some(next_physical) = (*block).next_physical {
        if next_physical.as_ref().is_free() {
          self.remove_free_block(next_physical.as_ptr(), self.binmap_down(next_physical.as_ref().size));
          (*block).size += next_physical.as_ref().size;
          (*block).next_physical = next_physical.as_ref().next_physical;
          if let Some(mut next_next) = (*block).next_physical {
            next_next.as_mut().prev_physical = Some(NonNull::new(block).unwrap());
          }
          System.dealloc(next_physical.as_ptr() as *mut u8, Layout::new::<Block>());
        }
      }
    }
  }

  fn binmap_down(&self, size: usize) -> BlockMap {
    let bin_idx = bit_scan_msb(size | Self::MIN_ALLOC_SIZE) as usize;
    let log2_subbin_size = bin_idx as usize - Self::SUB_BIN as usize;
    let sub_bin_idx = size >> log2_subbin_size;

    BlockMap {
      bin_idx: (bin_idx - Self::LINEAR as usize + (sub_bin_idx >> Self::SUB_BIN)) as usize,
      sub_bin_idx: (sub_bin_idx & (Self::SUB_BIN_COUNT - 1)) as usize,
      rounded_size: size,
      idx: (bin_idx - Self::LINEAR as usize + (sub_bin_idx >> Self::SUB_BIN)) * Self::SUB_BIN_COUNT
        + (sub_bin_idx & (Self::SUB_BIN_COUNT - 1)) as usize,
    }
  }

  fn binmap_up(&self, size: usize) -> BlockMap {
    let bin_idx = bit_scan_msb(size | Self::MIN_ALLOC_SIZE) as usize;
    let log2_subbin_size = bin_idx as usize - Self::SUB_BIN as usize;
    let next_subbin_offset = (1 << log2_subbin_size) - 1;
    let rounded = size + next_subbin_offset;
    let sub_bin_idx = rounded >> log2_subbin_size;

    BlockMap {
      bin_idx: (bin_idx - Self::LINEAR as usize + (sub_bin_idx >> Self::SUB_BIN)) as usize,
      sub_bin_idx: (sub_bin_idx & (Self::SUB_BIN_COUNT - 1)) as usize,
      rounded_size: rounded & !next_subbin_offset,
      idx: (bin_idx - Self::LINEAR as usize + (sub_bin_idx >> Self::SUB_BIN)) * Self::SUB_BIN_COUNT
        + (sub_bin_idx & (Self::SUB_BIN_COUNT - 1)) as usize,
    }
  }
}

// Align a number to the next multiple of a given alignment
fn align_forward(offset: usize, alignment: usize) -> usize {
  (offset + alignment - 1) & !(alignment - 1)
}

// Find the index of the most significant bit set in a number
fn bit_scan_msb(mask: usize) -> u32 {
  63 - mask.leading_zeros()
}

// Check if a number is a power of two
fn is_pow_two(num: usize) -> bool {
  (num & (num - 1)) == 0 && num > 0
}

unsafe impl GlobalAlloc for SpeedAllocator {
  unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
    let mut allocator = SpeedAllocator::new();
    if let Some(ptr) = allocator.allocate(layout.size(), layout.align()) {
      ptr.as_ptr()
    } else {
      null_mut()
    }
  }

  unsafe fn dealloc(&self, ptr: *mut u8, _: Layout) {
    let mut allocator = SpeedAllocator::new();
    if !ptr.is_null() {
      allocator.deallocate(NonNull::new_unchecked(ptr));
    }
  }
}
