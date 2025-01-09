#![no_std]

use allocator::{BaseAllocator, ByteAllocator, PageAllocator};

/// Early memory allocator
/// Use it before formal bytes-allocator and pages-allocator can work!
/// This is a double-end memory range:
/// - Alloc bytes forward
/// - Alloc pages backward
///
/// [ bytes-used | avail-area | pages-used ]
/// |            | -->    <-- |            |
/// start       b_pos        p_pos       end
///
/// For bytes area, 'count' records number of allocations.
/// When it goes down to ZERO, free bytes-used area.
/// For pages area, it will never be freed!
///
use allocator::AllocError;
pub type AllocResult<T = ()> = Result<T, AllocError>;
use core::alloc::Layout;
use core::ptr::NonNull;
pub struct EarlyAllocator<const PAGE_SIZE: usize> {
    start: usize,
    end: usize,
    b_pos: usize,
    p_pos: usize,
    count: usize,
    page_size: usize,
}

impl<const PAGE_SIZE: usize> EarlyAllocator<PAGE_SIZE> {
    pub const fn new() -> Self {
        Self {
            start: 0,
            end: 0,
            b_pos: 0,
            p_pos: 0,
            count: 0,
            page_size: PAGE_SIZE,
        }
    }
    fn check_overlap(&self, new_start: usize, new_size: usize) -> Result<(), AllocError> {
        // if new_start < self.start || (new_start + new_size) > self.end {
        //     return Err(AllocError::MemoryOverlap);
        // }
        // if new_start < self.bytes_pos && new_start + new_size > self.bytes_pos {
        //     return Err(AllocError::MemoryOverlap);
        // }
        // if new_start < self.end - self.pages_pos * self.page_size && new_start + new_size > self.end - self.pages_pos * self.page_size {
        //     return Err(AllocError::MemoryOverlap);
        // }
        if new_start < self.start || new_start + new_size > self.end {
            return Err(AllocError::MemoryOverlap);
        }
        if new_start < self.b_pos && new_start + new_size > self.b_pos {
            return Err(AllocError::MemoryOverlap);
        }
        if new_start < self.p_pos && new_start + new_size > self.p_pos {
            return Err(AllocError::MemoryOverlap);
        }
        Ok(())
    }
}

impl<const PAGE_SIZE: usize> BaseAllocator for EarlyAllocator<PAGE_SIZE> {
    fn init(&mut self, start: usize, size: usize) {
        self.start = start;
        self.end = start + size;
        self.b_pos = start;
        self.p_pos = self.end;
        self.count = 0;
    }

    fn add_memory(&mut self, start: usize, size: usize) -> AllocResult {
        self.check_overlap(start, size)?;
        if self.start == 0 && self.end == 0 {
            self.init(start, size);
        } else {
            return Err(AllocError::MemoryOverlap);
        }
        Ok(())
    }
}

impl<const PAGE_SIZE: usize> ByteAllocator for EarlyAllocator<PAGE_SIZE> {
    fn alloc(&mut self, layout: Layout) -> AllocResult<NonNull<u8>> {
        let size = layout.size();
        let align = layout.align();
        if size == 0 || align & (align - 1) != 0 || size & (align - 1) != 0 {
            return Err(AllocError::InvalidParam);
        }
        let align_mask = align - 1;
        let new_pos = (self.b_pos + align_mask) & !align_mask;
        if new_pos + size > self.p_pos {
            return Err(AllocError::NoMemory);
        }
        self.b_pos = new_pos + size;
        self.count += 1;
        Ok(NonNull::new(new_pos as *mut u8).unwrap())
    }

    fn dealloc(&mut self, pos: NonNull<u8>, layout: Layout) {
    }

    fn total_bytes(&self) -> usize {
        self.end - self.start
    }

    fn used_bytes(&self) -> usize {
        self.b_pos - self.start
    }

    fn available_bytes(&self) -> usize {
        self.p_pos - self.b_pos
    }
}

impl<const PAGE_SIZE: usize> PageAllocator for EarlyAllocator<PAGE_SIZE> {
    const PAGE_SIZE: usize = PAGE_SIZE;

    fn alloc_pages(&mut self, num_pages: usize, align_pow2: usize) -> AllocResult<usize> {
        let align_mask = align_pow2 - 1;
        let new_pos = (self.p_pos - num_pages * PAGE_SIZE + align_mask) & !align_mask;
        if new_pos < self.b_pos {
            return Err(AllocError::NoMemory);
        }
        self.p_pos = new_pos;
        Ok(new_pos)
    }

    fn dealloc_pages(&mut self, pos: usize, num_pages: usize) {
    }

    fn total_pages(&self) -> usize {
        self.total_bytes() / PAGE_SIZE
    }

    fn used_pages(&self) -> usize {
        (self.end - self.p_pos + PAGE_SIZE - 1) / PAGE_SIZE
    }

    fn available_pages(&self) -> usize {
        (self.p_pos - self.b_pos) / PAGE_SIZE
    }

    
}
