use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct RingBuffer<T> {
    buffer: Box<[MaybeUninit<T>]>,
    mask: usize,
    head: AtomicUsize,
    tail: AtomicUsize,
}

impl<T> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity.is_power_of_two());
        let mut buffer = Vec::with_capacity(capacity);
        buffer.resize_with(capacity, || MaybeUninit::uninit());
        Self {
            buffer: buffer.into_boxed_slice(),
            mask: capacity - 1,
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    pub fn push(&mut self, value: T) -> bool {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);
        if head.wrapping_sub(tail) == self.buffer.len() {
            return false;
        }
        let idx = head & self.mask;
        unsafe {
            self.buffer[idx].as_mut_ptr().write(value);
        }
        self.head.store(head.wrapping_add(1), Ordering::Release);
        true
    }

    pub fn pop(&mut self) -> Option<T> {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);
        if tail == head {
            return None;
        }
        let idx = tail & self.mask;
        let value = unsafe { self.buffer[idx].as_ptr().read() };
        self.tail.store(tail.wrapping_add(1), Ordering::Release);
        Some(value)
    }

    pub fn is_empty(&self) -> bool {
        self.tail.load(Ordering::Acquire) == self.head.load(Ordering::Acquire)
    }

    pub fn len(&self) -> usize {
        self.head
            .load(Ordering::Acquire)
            .wrapping_sub(self.tail.load(Ordering::Acquire))
    }

    pub fn capacity(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_full(&self) -> bool {
        self.head
            .load(Ordering::Acquire)
            .wrapping_sub(self.tail.load(Ordering::Acquire))
            == self.buffer.len()
    }

    pub fn clear(&mut self) {
        while self.pop().is_some() {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_pop_basic() {
        let mut rb: RingBuffer<i32> = RingBuffer::new(8);
        assert!(rb.is_empty());
        assert_eq!(rb.len(), 0);

        assert!(rb.push(42));
        assert!(!rb.is_empty());
        assert_eq!(rb.len(), 1);

        assert_eq!(rb.pop(), Some(42));
        assert!(rb.is_empty());
    }

    #[test]
    fn test_push_pop_multiple() {
        let mut rb: RingBuffer<i32> = RingBuffer::new(4);

        assert!(rb.push(1));
        assert!(rb.push(2));
        assert!(rb.push(3));

        assert_eq!(rb.pop(), Some(1));
        assert_eq!(rb.pop(), Some(2));
        assert_eq!(rb.pop(), Some(3));
        assert!(rb.is_empty());
    }

    #[test]
    fn test_ring_buffer_overflow() {
        let mut rb: RingBuffer<i32> = RingBuffer::new(2);

        assert!(rb.push(1));
        assert!(rb.push(2));
        assert!(!rb.push(3));

        assert_eq!(rb.pop(), Some(1));
        assert!(rb.push(3));
    }

    #[test]
    fn test_complex_types() {
        let mut rb: RingBuffer<String> = RingBuffer::new(4);

        rb.push("hello".to_string());
        rb.push("world".to_string());

        assert_eq!(rb.pop(), Some("hello".to_string()));
        assert_eq!(rb.pop(), Some("world".to_string()));
    }
}
