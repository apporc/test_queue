use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use ringbuf::{StaticRb, traits::*};
use ringbuf_blocking::BlockingStaticRb;
use std::thread;
use test_queue::queue::Queue;
use tokio::runtime::Runtime;
use tokio::sync::mpsc as tokio_mpsc;

// 基准测试函数
fn bench_queues(c: &mut Criterion) {
    const MSG_COUNT: usize = 1024;

    // 测试自定义队列
    let mut group = c.benchmark_group("Queue Throughput");
    group.throughput(Throughput::Elements(MSG_COUNT as u64));
    group.measurement_time(std::time::Duration::from_secs(30));
    group.sample_size(1000);

    group.bench_function("ringbuf async", |b| {
        // 每次迭代创建新的队列和运行时
        b.iter(|| {
            let (mut tx, mut rx) = Queue::<u64, 1024>::new();
            let rt = Runtime::new().unwrap();

            // 生产者线程
            let producer = thread::spawn(move || {
                for _ in 0..MSG_COUNT {
                    tx.send(0).unwrap(); // 明确指定u64类型
                }
            });

            // 消费者任务
            rt.block_on(async {
                for _ in 0..MSG_COUNT {
                    black_box(rx.recv().await);
                }
            });

            producer.join().unwrap();
        });
    });

    // 测试flume
    group.bench_function("FlumeMPSC", |b| {
        // 每次迭代创建新的队列和运行时
        b.iter(|| {
            let (tx, rx) = flume::bounded::<u64>(1024); // 明确指定u64类型
            let rt = Runtime::new().unwrap();

            // 生产者线程
            let producer = thread::spawn(move || {
                for _ in 0..MSG_COUNT {
                    tx.send(0).unwrap(); // 明确指定u64类型
                }
            });

            // 消费者任务
            rt.block_on(async {
                for _ in 0..MSG_COUNT {
                    black_box(rx.recv_async().await.unwrap());
                }
            });

            producer.join().unwrap();
        });
    });

    // 测试tokio::sync::mpsc
    group.bench_function("TokioMPSC", |b| {
        // 每次迭代创建新的队列和运行时
        b.iter(|| {
            let rt = Runtime::new().unwrap();
            let (tx, mut rx) = tokio_mpsc::channel::<u64>(1024); // 明确指定u64类型

            // 生产者线程
            let producer = thread::spawn(move || {
                let rt = Runtime::new().unwrap();
                rt.block_on(async {
                    for _ in 0..MSG_COUNT {
                        tx.send(0).await.unwrap(); // 明确指定u64类型
                    }
                });
            });

            // 消费者任务
            rt.block_on(async {
                for _ in 0..MSG_COUNT {
                    black_box(rx.recv().await.unwrap());
                }
            });

            producer.join().unwrap();
        });
    });

    group.bench_function("TokioMPSC coroutine", |b| {
        b.iter(|| {
            // 创建单一运行时
            let rt = Runtime::new().unwrap();
            let (tx, mut rx) = tokio_mpsc::channel::<u64>(1024);

            // 在同一运行时中运行生产者和消费者
            rt.block_on(async {
                // 生产者协程
                let producer = tokio::spawn(async move {
                    for _ in 0..MSG_COUNT {
                        tx.send(0).await.unwrap();
                    }
                });

                // 消费者协程
                for _ in 0..MSG_COUNT {
                    black_box(rx.recv().await.unwrap());
                }

                // 等待生产者完成
                producer.await.unwrap();
            });
        });
    });

    group.bench_function("Flume (sync)", |b| {
        b.iter(|| {
            let (tx, rx) = flume::bounded(1024);

            let producer = thread::spawn(move || {
                for _ in 0..MSG_COUNT {
                    tx.send(0).unwrap();
                }
            });

            let consumer = thread::spawn(move || {
                for _ in 0..MSG_COUNT {
                    black_box(rx.recv().unwrap());
                }
            });

            producer.join().unwrap();
            consumer.join().unwrap();
        });
    });

    group.bench_function("Std MPSC (sync)", |b| {
        b.iter(|| {
            let (tx, rx) = std::sync::mpsc::sync_channel(1024);

            let producer = thread::spawn(move || {
                for _ in 0..MSG_COUNT {
                    tx.send(0).unwrap();
                }
            });

            let consumer = thread::spawn(move || {
                for _ in 0..MSG_COUNT {
                    black_box(rx.recv().unwrap());
                }
            });

            producer.join().unwrap();
            consumer.join().unwrap();
        });
    });

    group.bench_function("ringbuf (sync)", |b| {
        b.iter(|| {
            let rb = StaticRb::<i32, 1024>::default();
            let (tx, rx) = rb.split();

            let producer = thread::spawn(move || {
                let mut tx = tx;
                for _ in 0..MSG_COUNT {
                    let _ = tx.try_push(0);
                }
            });

            let consumer = thread::spawn(move || {
                let mut rx = rx;
                for _ in 0..MSG_COUNT {
                    black_box(rx.try_pop());
                }
            });

            producer.join().unwrap();
            consumer.join().unwrap();
        });
    });

    group.bench_function("ringbuf blocking (sync)", |b| {
        b.iter(|| {
            // 使用 HeapRb 作为存储，容量为 1024
            let rb = BlockingStaticRb::<i32, 1024>::default();

            let (mut tx, mut rx) = rb.split();

            let producer = thread::spawn(move || {
                for _ in 0..MSG_COUNT {
                    tx.push(0).unwrap(); // 阻塞推送
                }
            });

            let consumer = thread::spawn(move || {
                for _ in 0..MSG_COUNT {
                    black_box(rx.pop().unwrap()); // 阻塞弹出
                }
            });

            producer.join().unwrap();
            consumer.join().unwrap();
        });
    });

    group.finish();
}

criterion_group!(benches, bench_queues);
criterion_main!(benches);
