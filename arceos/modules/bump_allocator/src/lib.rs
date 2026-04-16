#![no_std]

use allocator::{BaseAllocator, ByteAllocator, PageAllocator, AllocResult, AllocError};
use core::alloc::Layout;
use core::ptr::NonNull;
use core::cmp::max;

/// A simple bump allocator.
pub struct BumpAllocator {
    start: usize,
    end: usize,
    next: usize,
}

impl BumpAllocator {
    pub const fn new() -> Self {
        Self {
            start: 0,
            end: 0,
            next: 0,
        }
    }

    fn init_inner(&mut self, start: usize, size: usize) {
        self.start = start;
        self.end = start + size;
        self.next = start;
    }

    fn add_memory_inner(&mut self, start: usize, size: usize) -> AllocResult {
        if self.start == 0 && self.end == 0 {
            self.init_inner(start, size);
            Ok(())
        } else if start == self.end {
            self.end += size;
            Ok(())
        } else {
            // Standard bump allocator doesn't support non-contiguous memory easily.
            // But we can just allow it if it happens to be contiguous with our current range.
            // Otherwise, we do nothing for now as per simple bump logic.
            Ok(())
        }
    }
}

impl BaseAllocator for BumpAllocator {
    fn init(&mut self, start: usize, size: usize) {
        self.init_inner(start, size);
    }

    fn add_memory(&mut self, start: usize, size: usize) -> AllocResult {
        self.add_memory_inner(start, size)
    }
}

impl ByteAllocator for BumpAllocator {
    fn alloc(&mut self, layout: Layout) -> AllocResult<NonNull<u8>> {
        let align = layout.align();
        let size = layout.size();
        let start = (self.next + align - 1) & !(align - 1);
        let end = start + size;
        if end <= self.end {
            self.next = end;
            NonNull::new(start as *mut u8).ok_or(AllocError::NoMemory)
        } else {
            Err(AllocError::NoMemory)
        }
    }

    fn dealloc(&mut self, _pos: NonNull<u8>, _layout: Layout) {
        // Bump allocator does not support individual deallocation.
    }

    fn total_bytes(&self) -> usize {
        self.end - self.start
    }

    fn used_bytes(&self) -> usize {
        self.next - self.start
    }

    fn available_bytes(&self) -> usize {
        self.end - self.next
    }
}

impl PageAllocator for BumpAllocator {
    const PAGE_SIZE: usize = 4096;

    fn alloc_pages(&mut self, num_pages: usize, align_pow2: usize) -> AllocResult<usize> {
        let align = max(1 << align_pow2, Self::PAGE_SIZE);
        let size = num_pages * Self::PAGE_SIZE;
        let start = (self.next + align - 1) & !(align - 1);
        let end = start + size;
        if end <= self.end {
            self.next = end;
            Ok(start)
        } else {
            Err(AllocError::NoMemory)
        }
    }

    fn dealloc_pages(&mut self, _pos: usize, _num_pages: usize) {
        // Bump allocator does not support individual deallocation.
    }

    fn total_pages(&self) -> usize {
        self.total_bytes() / Self::PAGE_SIZE
    }

    fn used_pages(&self) -> usize {
        (self.used_bytes() + Self::PAGE_SIZE - 1) / Self::PAGE_SIZE
    }

    fn available_pages(&self) -> usize {
        self.available_bytes() / Self::PAGE_SIZE
    }
}

/// Compatibility wrapper for EarlyAllocator.
pub struct EarlyAllocator<const SIZE: usize> {
    inner: BumpAllocator,
}

impl<const SIZE: usize> EarlyAllocator<SIZE> {
    pub const fn new() -> Self {
        Self {
            inner: BumpAllocator::new(),
        }
    }
}

impl<const SIZE: usize> BaseAllocator for EarlyAllocator<SIZE> {
    fn init(&mut self, start: usize, size: usize) {
        self.inner.init(start, size);
    }

    fn add_memory(&mut self, start: usize, size: usize) -> AllocResult {
        self.inner.add_memory(start, size)
    }
}

impl<const SIZE: usize> ByteAllocator for EarlyAllocator<SIZE> {
    fn alloc(&mut self, layout: Layout) -> AllocResult<NonNull<u8>> {
        self.inner.alloc(layout)
    }

    fn dealloc(&mut self, pos: NonNull<u8>, layout: Layout) {
        self.inner.dealloc(pos, layout)
    }

    fn total_bytes(&self) -> usize {
        self.inner.total_bytes()
    }

    fn used_bytes(&self) -> usize {
        self.inner.used_bytes()
    }

    fn available_bytes(&self) -> usize {
        self.inner.available_bytes()
    }
}

impl<const SIZE: usize> PageAllocator for EarlyAllocator<SIZE> {
    const PAGE_SIZE: usize = SIZE;

    fn alloc_pages(&mut self, num_pages: usize, align_pow2: usize) -> AllocResult<usize> {
        // Page allocation for EarlyAllocator using its own PAGE_SIZE
        let align = max(1 << align_pow2, Self::PAGE_SIZE);
        let size = num_pages * Self::PAGE_SIZE;
        let start = (self.inner.next + align - 1) & !(align - 1);
        let end = start + size;
        if end <= self.inner.end {
            self.inner.next = end;
            Ok(start)
        } else {
            Err(AllocError::NoMemory)
        }
    }

    fn dealloc_pages(&mut self, pos: usize, num_pages: usize) {
        self.inner.dealloc_pages(pos, num_pages)
    }

    fn total_pages(&self) -> usize {
        self.inner.total_bytes() / Self::PAGE_SIZE
    }

    fn used_pages(&self) -> usize {
        (self.inner.used_bytes() + Self::PAGE_SIZE - 1) / Self::PAGE_SIZE
    }

    fn available_pages(&self) -> usize {
        self.inner.available_bytes() / Self::PAGE_SIZE
    }
}