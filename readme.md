### SpeedAllocator Overview

The Two-Level Segregated Fit (SpeedAllocator) allocator is designed for real-time systems, providing constant and predictable response times for memory allocation and deallocation, while minimizing fragmentation. 

#### Key Features

1. **Two-Level Segregated Free Lists**:
   - **First-Level Index (FLI)**: Divides memory blocks into size classes that are powers of two (e.g., 16, 32, 64 bytes).
   - **Second-Level Index (SLI)**: Subdivides each first-level class into smaller size ranges to reduce fragmentation.

2. **Block Header**:
   - `size`: Size of the block.
   - `prev_phys_block`: Pointer to the previous physical block.
   - `next_free`: Pointer to the next free block in the free list.
   - `prev_free`: Pointer to the previous free block in the free list.
   - `is_free`: Indicates if the block is free.
   - `is_last`: Indicates if this is the last block in the memory pool.

3. **Bitmaps**: Used to quickly locate non-empty free lists, optimizing search operations.

#### SpeedAllocator Structure Diagram

```plaintext
|  SpeedAllocator Structure  |
|____________________________|
|                            |
| First-Level Array (FLI)    |
|  ________________________  |
| |     |     |     |     |  |
| | FLI | FLI | ... | FLI |  |
|_|_____|_____|_____|_____|__|
      |     |     |     |      
      v     v     v     v      
|____________________________|
| Second-Level Array (SLI)   |
|  ________________________  |
| | SLI | SLI | ... | SLI |  |
|_|_____|_____|_____|_____|__|
      |     |     |     |      
      v     v     v     v      
|____________________________|
|       Free Lists           |
|  ________________________  |
| | blk | blk | ... | blk |  |
|_|_____|_____|_____|_____|__|

Each entry in the first-level array points to a second-level array, which in turn contains lists of free memory blocks. The combination of these arrays allows for efficient management and quick access to free blocks of various sizes.
```

#### Basic Operations

1. **Allocation (`allocate`)**:
   - Find a suitable free block using the first and second-level indices.
   - Split the block if necessary.
   - Update the free lists and bitmaps accordingly.

2. **Deallocation (`deallocate`)**:
   - Mark the block as free.
   - Coalesce (merge) with adjacent free blocks if possible.
   - Insert the block back into the appropriate free list and update the bitmaps.

#### How It Works

1. **Mapping Function**: Determines the first and second-level indices based on the size of the block.
2. **Insert Free Block**: Adds a block to the appropriate free list.
3. **Remove Free Block**: Removes a block from the free list.
4. **Coalescing**: Merges adjacent free blocks to reduce fragmentation.

The SpeedAllocator  is efficient and predictable, making it suitable for real-time systems where predictable behavior and performance are critical. The use of two-level segregated lists and bitmaps ensures fast and constant-time operations for both allocation and deallocation.

## Research Paper

- [Two-Level Segregated Fit Allocator](http://www.gii.upv.es/tlsf/files/papers/ecrts04_tlsf.pdf)
