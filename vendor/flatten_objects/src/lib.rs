//! A simple object pool for storing objects in a flattened array.

#![no_std]

use core::mem::MaybeUninit;
use core::default::Default;
use core::ops::Drop;
use core::marker::Send;
use core::marker::Sync;
use core::option::Option::{self, None, Some};

const CAPACITY: usize = 64;

/// A simple object pool that stores objects in a flattened array.
pub struct FlattenObjects<T, const N: usize = CAPACITY> {
    objects: [MaybeUninit<T>; N],
    bitmap: u64,
}

unsafe impl<T: Send, const N: usize> Send for FlattenObjects<T, N> {}
unsafe impl<T: Sync, const N: usize> Sync for FlattenObjects<T, N> {}

impl<T, const N: usize> FlattenObjects<T, N> {
    /// Creates a new empty object pool.
    pub const fn new() -> Self {
        // 使用 MaybeUninit::uninit() 替代 uninit_array()
        let objects: [MaybeUninit<T>; N] = unsafe { MaybeUninit::uninit().assume_init() };

        FlattenObjects {
            objects,
            bitmap: 0,
        }
    }

    /// Inserts a new object into the pool and returns its index.
    pub fn insert(&mut self, value: T) -> Option<usize> {
        if N > 64 {
            return None; // 不支持超过64的容量
        }
        let index = self.first_zero()?;
        self.bitmap |= 1u64 << index;
        unsafe {
            self.objects[index].as_mut_ptr().write(value);
        }
        Some(index)
    }

    /// Adds a new object into the pool and returns its index (alias for insert).
    pub fn add(&mut self, value: T) -> Option<usize> {
        self.insert(value)
    }

    /// Adds a new object at the specified index.
    pub fn add_at(&mut self, index: usize, value: T) -> Option<()> {
        if index >= N || (self.bitmap & (1u64 << index)) != 0 {
            return None;
        }
        self.bitmap |= 1u64 << index;
        unsafe {
            self.objects[index].as_mut_ptr().write(value);
        }
        Some(())
    }

    /// Removes and returns the object at the given index.
    pub fn remove(&mut self, index: usize) -> Option<T> {
        if index >= N || (self.bitmap & (1u64 << index)) == 0 {
            return None;
        }
        self.bitmap &= !(1u64 << index);
        unsafe { Some(self.objects[index].as_ptr().read()) }
    }

    /// Returns a reference to the object at the given index.
    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= N || (self.bitmap & (1u64 << index)) == 0 {
            return None;
        }
        unsafe { Some(&*self.objects[index].as_ptr()) }
    }

    /// Returns a mutable reference to the object at the given index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index >= N || (self.bitmap & (1u64 << index)) == 0 {
            return None;
        }
        unsafe { Some(&mut *self.objects[index].as_mut_ptr()) }
    }

    /// Returns the number of objects in the pool.
    pub fn len(&self) -> usize {
        self.bitmap.count_ones() as usize
    }

    /// Returns true if the pool is empty.
    pub fn is_empty(&self) -> bool {
        self.bitmap == 0
    }

    /// Returns the capacity of the pool.
    pub const fn capacity(&self) -> usize {
        N
    }

    /// Clears all objects from the pool.
    pub fn clear(&mut self) {
        for index in 0..N {
            if (self.bitmap & (1u64 << index)) != 0 {
                self.bitmap &= !(1u64 << index);
                unsafe {
                    core::ptr::drop_in_place(self.objects[index].as_mut_ptr());
                }
            }
        }
    }

    /// Finds the first zero bit in the bitmap.
    fn first_zero(&self) -> Option<usize> {
        if N > 64 {
            return None;
        }
        let mask = (1u64 << N) - 1;
        let inverted = !self.bitmap & mask;
        if inverted == 0 {
            None
        } else {
            Some(inverted.trailing_zeros() as usize)
        }
    }
}

impl<T, const N: usize> Drop for FlattenObjects<T, N> {
    fn drop(&mut self) {
        self.clear();
    }
}

impl<T, const N: usize> Default for FlattenObjects<T, N> {
    fn default() -> Self {
        Self::new()
    }
}
