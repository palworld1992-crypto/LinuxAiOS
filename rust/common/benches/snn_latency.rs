//! SNN Latency Benchmark
//! Measures SNN spike processing latency using criterion
//! Run with: cargo bench --features benchmark -p common

use common::ring_buffer::RingBuffer;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use parking_lot::RwLock;
use std::sync::Arc;

const SPIKE_RING_SIZE: usize = 4096;

#[derive(Clone, Copy, Debug)]
struct SpikeEvent {
    pub pid: u32,
    pub vaddr: u64,
    pub timestamp_ns: u64,
}

struct LifNeuron {
    v: f32,
    leak: f32,
    threshold: f32,
    reset: f32,
    weight: f32,
}

impl LifNeuron {
    fn new() -> Self {
        Self {
            v: 0.0,
            leak: 0.95,
            threshold: 1.0,
            reset: 0.0,
            weight: 1.0,
        }
    }

    #[inline(always)]
    fn step(&mut self, input: f32) -> bool {
        self.v = self.v * self.leak + input * self.weight;
        if self.v >= self.threshold {
            self.v = self.reset;
            true
        } else {
            false
        }
    }
}

fn bench_neuron_step(c: &mut Criterion) {
    let mut group = c.benchmark_group("snn_neuron");

    let mut neuron = LifNeuron::new();
    let input = black_box(1.0f32);

    group.bench_function("step", |b| {
        b.iter(|| {
            neuron.v = 0.0; // Reset
            black_box(neuron.step(input))
        });
    });

    group.finish();
}

fn bench_ring_buffer_push(c: &mut Criterion) {
    let mut group = c.benchmark_group("ring_buffer");

    let mut rb = RingBuffer::<SpikeEvent>::new(SPIKE_RING_SIZE);
    let event = black_box(SpikeEvent {
        pid: 1234,
        vaddr: 0x7f000000,
        timestamp_ns: 1234567890,
    });

    group.bench_function("push", |b| {
        b.iter(|| {
            // Reset if full
            if !rb.push(event) {
                let _ = rb.pop();
                let _ = rb.push(event);
            }
            black_box(())
        });
    });

    group.finish();
}

fn bench_ring_buffer_pop(c: &mut Criterion) {
    let mut group = c.benchmark_group("ring_buffer");

    // Pre-fill ring buffer
    let mut rb = RingBuffer::<SpikeEvent>::new(SPIKE_RING_SIZE);
    let event = SpikeEvent {
        pid: 1234,
        vaddr: 0x7f000000,
        timestamp_ns: 1234567890,
    };
    let _ = rb.push(event);

    group.bench_function("pop", |b| {
        b.iter(|| {
            // Push back if empty
            if rb.pop().is_none() {
                let _ = rb.push(event);
            }
            black_box(())
        });
    });

    group.finish();
}

fn bench_dashmap_operations(c: &mut Criterion) {
    use dashmap::DashMap;

    let mut group = c.benchmark_group("dashmap");

    let map = Arc::new(DashMap::new());

    // Pre-populate
    for i in 0..1000 {
        map.insert(i, i * 2);
    }

    group.bench_function("insert", |b| {
        let counter = std::sync::atomic::AtomicUsize::new(1000);
        b.iter(|| {
            let key = counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            black_box(map.insert(key, key));
        });
    });

    group.bench_function("get", |b| {
        b.iter(|| {
            for i in (0..1000).step_by(10) {
                black_box(map.get(&i));
            }
        });
    });

    group.bench_function("remove", |b| {
        let counter = std::sync::atomic::AtomicUsize::new(0);
        b.iter(|| {
            let key = counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % 1000;
            black_box(map.remove(&key));
        });
    });

    group.finish();
}

fn bench_spike_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("snn_full_path");

    // Setup: 64 neurons
    let neurons = Arc::new(RwLock::new(
        (0..64).map(|_| LifNeuron::new()).collect::<Vec<_>>(),
    ));
    let spike_rb = Arc::new(parking_lot::Mutex::new(RingBuffer::<SpikeEvent>::new(
        SPIKE_RING_SIZE,
    )));

    // Pre-fill with spike events
    {
        let mut rb = spike_rb.lock();
        for i in 0..100 {
            let _ = rb.push(SpikeEvent {
                pid: 1234,
                vaddr: 0x7f000000 + i * 4096,
                timestamp_ns: i as u64,
            });
        }
    }

    group.bench_function("process_10_spikes", |b| {
        b.iter(|| {
            let mut rb = spike_rb.lock();
            let neuron_len = neurons.read().len();

            for _ in 0..10 {
                if let Some(event) = rb.pop() {
                    let idx = (event.vaddr as usize) % neuron_len;
                    let mut neurons_guard = neurons.write();
                    let neuron = &mut neurons_guard[idx];
                    black_box(neuron.step(1.0));

                    // Satisfy Rule 0: use pid and timestamp
                    black_box(event.pid);
                    black_box(event.timestamp_ns);

                    // Push back for next iteration
                    let _ = rb.push(event);
                }
            }
        });
    });

    group.finish();
}

fn bench_latency_micro(c: &mut Criterion) {
    // Measure raw latency in microseconds
    let mut group = c.benchmark_group("latency_micro");

    group.bench_function("neuron_step_raw", |b| {
        let mut neuron = LifNeuron::new();
        b.iter(|| {
            neuron.v = 0.0;
            neuron.step(black_box(1.0))
        });
    });

    // Target: < 1 μs per spike
    group.measurement_time(std::time::Duration::from_secs(5));
    group.sample_size(10000);

    group.finish();
}

criterion_group!(
    benches,
    bench_neuron_step,
    bench_ring_buffer_push,
    bench_ring_buffer_pop,
    bench_dashmap_operations,
    bench_spike_processing,
    bench_latency_micro
);
criterion_main!(benches);
