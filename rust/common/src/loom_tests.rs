//! Loom tests for lock-free data structures in Common module.
//! Run with: cargo test -p common --features "loom"

#[cfg(feature = "loom")]
mod tests {
    use loom::model::CheckOutcome;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use parking_lot::Mutex;
    use common::ring_buffer::RingBuffer;
    use dashmap::DashMap;

    /// Test RingBuffer with concurrent push/pop operations
    /// Uses Mutex to share the ring buffer between threads
    #[test]
    fn test_ring_buffer_concurrent() {
        loom::model::Builder::new()
            .spawn(move || {
                let rb = Arc::new(Mutex::new(RingBuffer::<u64>::new(8)));
                let rb_producer = rb.clone();
                let rb_consumer = rb.clone();

                // Producer thread
                let producer = loom::thread::spawn(move || {
                    let mut guard = rb_producer.lock();
                    for i in 0..4 {
                        let _ = guard.push(i);
                    }
                });

                // Consumer thread
                let consumer = loom::thread::spawn(move || {
                    let mut guard = rb_consumer.lock();
                    for _ in 0..4 {
                        let _ = guard.pop();
                    }
                });

                producer.join().ok();
                consumer.join().ok();
            })
            .unwrap()
            .check();
    }

    /// Test DashMap with concurrent insert/read operations
    #[test]
    fn test_dashmap_concurrent() {
        loom::model::Builder::new()
            .spawn(move || {
                let map: Arc<DashMap<u64, u64>> = Arc::new(DashMap::new());
                let map_clone = map.clone();

                // Writer thread
                let writer = loom::thread::spawn(move || {
                    for i in 0..10 {
                        map.insert(i, i * 2);
                    }
                });

                // Reader thread
                let reader = loom::thread::spawn(move || {
                    for _ in 0..10 {
                        for item in map_clone.iter() {
                            let _ = item.key();
                            let _ = item.value();
                        }
                    }
                });

                writer.join().ok();
                reader.join().ok();
            })
            .unwrap()
            .check();
    }

    /// Test atomic counter for reference counting
    #[test]
    fn test_atomic_ref_count() {
        loom::model::Builder::new()
            .spawn(move || {
                let counter = Arc::new(AtomicUsize::new(1));
                let counter_clone = counter.clone();

                let inc = loom::thread::spawn(move || {
                    counter.fetch_add(1, Ordering::SeqCst);
                });

                let dec = loom::thread::spawn(move || {
                    counter_clone.fetch_sub(1, Ordering::SeqCst);
                });

                inc.join().ok();
                dec.join().ok();

                let val = counter.load(Ordering::SeqCst);
                assert!(val >= 1);
            })
            .unwrap()
            .check();
    }
}

#[cfg(not(feature = "loom"))]
mod tests {
    #[test]
    fn test_loom_disabled() {
        println!("Run with --features loom to enable loom tests");
    }
}
                });

                // Consumer thread
                let consumer = loom::thread::spawn(move || {
                    for _ in 0..4 {
                        let _ = rb_clone.pop();
                    }
                });

                producer.join().ok();
                consumer.join().ok();
            })
            .unwrap()
            .check();
    }

    /// Test DashMap with concurrent insert/read operations
    #[test]
    fn test_dashmap_concurrent() {
        loom::model::Builder::new()
            .spawn(move || {
                let map: Arc<DashMap<u64, u64>> = Arc::new(DashMap::new());
                let map_clone = map.clone();

                // Writer thread
                let writer = loom::thread::spawn(move || {
                    for i in 0..10 {
                        map.insert(i, i * 2);
                    }
                });

                // Reader thread
                let reader = loom::thread::spawn(move || {
                    for _ in 0..10 {
                        for item in map_clone.iter() {
                            let _ = item.key();
                            let _ = item.value();
                        }
                    }
                });

                writer.join().ok();
                reader.join().ok();
            })
            .unwrap()
            .check();
    }

    /// Test DashMap concurrent remove operation
    #[test]
    fn test_dashmap_remove_concurrent() {
        loom::model::Builder::new()
            .spawn(move || {
                let map: Arc<DashMap<u64, u64>> = Arc::new(DashMap::new());
                map.insert(1, 100);
                map.insert(2, 200);

                let map_clone = map.clone();

                let remover = loom::thread::spawn(move || {
                    map.remove(&1);
                });

                let reader = loom::thread::spawn(move || {
                    let _ = map_clone.get(&1);
                    let _ = map_clone.get(&2);
                });

                remover.join().ok();
                reader.join().ok();
            })
            .unwrap()
            .check();
    }

    /// Test atomic counter for reference counting
    #[test]
    fn test_atomic_ref_count() {
        loom::model::Builder::new()
            .spawn(move || {
                let counter = Arc::new(AtomicUsize::new(1));
                let counter_clone = counter.clone();

                let inc = loom::thread::spawn(move || {
                    counter.fetch_add(1, Ordering::SeqCst);
                });

                let dec = loom::thread::spawn(move || {
                    counter_clone.fetch_sub(1, Ordering::SeqCst);
                });

                inc.join().ok();
                dec.join().ok();

                let val = counter.load(Ordering::SeqCst);
                assert!(val >= 1);
            })
            .unwrap()
            .check();
    }
}

#[cfg(not(feature = "loom"))]
mod tests {
    #[test]
    fn test_loom_disabled() {
        println!("Run with --features loom to enable loom tests");
    }
}
