//! The bounded hand-off from the network thread to the main thread.
//!
//! The network thread pushes decoded calls; the `FEngineLoop::Tick` detour
//! pops them. The lock is held only for the push or the pop itself, never
//! while calling into the game, and a full queue rejects the new item
//! rather than blocking the network thread.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

/// A fixed-capacity FIFO shared by one producer and one consumer thread.
#[derive(Debug)]
pub struct EventQueue<T> {
    items: Mutex<VecDeque<T>>,
    /// Mirrors `items.len()` so the per-frame "anything to do?" check on
    /// the main thread is one atomic load, with no lock.
    len: AtomicUsize,
    capacity: usize,
}

impl<T> EventQueue<T> {
    /// An empty queue holding at most `capacity` items.
    pub const fn new(capacity: usize) -> Self {
        Self {
            items: Mutex::new(VecDeque::new()),
            len: AtomicUsize::new(0),
            capacity,
        }
    }

    fn lock(&self) -> MutexGuard<'_, VecDeque<T>> {
        // Nothing panics while holding the lock, but a poisoned queue must
        // still work: losing the Black Market is worse than a stale flag.
        self.items.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Append `item`, or hand it back if the queue is full.
    pub fn push(&self, item: T) -> Result<(), T> {
        let mut items = self.lock();
        if items.len() >= self.capacity {
            return Err(item);
        }
        items.push_back(item);
        self.len.store(items.len(), Ordering::Release);
        Ok(())
    }

    /// Take the oldest item.
    pub fn pop(&self) -> Option<T> {
        let mut items = self.lock();
        let item = items.pop_front();
        self.len.store(items.len(), Ordering::Release);
        item
    }

    /// Items waiting. Lock-free; may be momentarily stale.
    pub fn len(&self) -> usize {
        self.len.load(Ordering::Acquire)
    }

    /// Whether nothing is waiting. Lock-free; may be momentarily stale.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The most items the queue holds.
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fifo_order() {
        let q = EventQueue::new(4);
        q.push(1).unwrap();
        q.push(2).unwrap();
        q.push(3).unwrap();
        assert_eq!(q.len(), 3);
        assert_eq!(q.pop(), Some(1));
        assert_eq!(q.pop(), Some(2));
        assert_eq!(q.pop(), Some(3));
        assert_eq!(q.pop(), None);
        assert!(q.is_empty());
    }

    /// A full queue hands the new item back and keeps the old ones: the
    /// producer drops and counts, it never blocks or evicts.
    #[test]
    fn full_queue_rejects_the_new_item() {
        let q = EventQueue::new(2);
        q.push("a").unwrap();
        q.push("b").unwrap();
        assert_eq!(q.push("c"), Err("c"));
        assert_eq!(q.len(), 2);
        assert_eq!(q.pop(), Some("a"));
        q.push("d").unwrap();
        assert_eq!(q.pop(), Some("b"));
        assert_eq!(q.pop(), Some("d"));
    }

    #[test]
    fn works_as_a_static() {
        static Q: EventQueue<u32> = EventQueue::new(1);
        Q.push(7).unwrap();
        assert_eq!(Q.pop(), Some(7));
        assert_eq!(Q.capacity(), 1);
    }

    /// One producer and one consumer thread lose nothing: every item pushed
    /// is either popped once or handed back as rejected.
    #[test]
    fn cross_thread_hand_off_loses_nothing() {
        let q = std::sync::Arc::new(EventQueue::new(8));
        let producer = {
            let q = q.clone();
            std::thread::spawn(move || {
                let mut rejected = 0u32;
                for i in 0..10_000u32 {
                    if q.push(i).is_err() {
                        rejected += 1;
                    }
                }
                rejected
            })
        };
        let mut popped = Vec::new();
        while !producer.is_finished() {
            if let Some(i) = q.pop() {
                popped.push(i);
            }
        }
        let rejected = producer.join().unwrap();
        // The lock, not the lock-free length, is authoritative here.
        while let Some(i) = q.pop() {
            popped.push(i);
        }
        assert_eq!(popped.len() as u32 + rejected, 10_000);
        assert!(
            popped.windows(2).all(|w| w[0] < w[1]),
            "FIFO across threads"
        );
    }
}
