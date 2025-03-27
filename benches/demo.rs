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
    group.sample_size(10);

    // 创建单一运行时
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2) // 限制为 2 个线程
        .thread_name("tokio-worker")
        .on_thread_start(|| {
            let handle = thread::current();
            println!("Thread {} started", handle.name().unwrap_or("unnamed"));
        })
        .enable_all()
        .build()
        .unwrap();

    group.bench_function("TokioMPSC multi-thread single-task", |b| {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        b.iter(|| {
            let (tx, mut rx) = tokio_mpsc::channel::<u64>(1024);
            rt.block_on(async {
                let task = tokio::spawn(async move {
                    let producer = async {
                        for _ in 0..MSG_COUNT {
                            tx.send(0).await.unwrap();
                        }
                    };
                    let consumer = async {
                        for _ in 0..MSG_COUNT {
                            black_box(rx.recv().await.unwrap());
                        }
                    };
                    tokio::join!(producer, consumer);
                });
                task.await.unwrap();
            });
        });
    });

    group.bench_function("TokioMPSC coroutine optimized", |b| {
        b.iter(|| {
            let (tx, mut rx) = tokio_mpsc::channel::<u64>(1024);
            rt.block_on(async {
                let producer = tokio::spawn(async move {
                    for _ in 0..MSG_COUNT {
                        tx.send(0).await.unwrap();
                    }
                });
                for _ in 0..MSG_COUNT {
                    black_box(rx.recv().await.unwrap());
                }
                producer.await.unwrap();
            });
        });
    });

    group.bench_function("TokioMPSC parallel", |b| {
        b.iter(|| {
            let (tx, mut rx) = tokio_mpsc::channel::<u64>(1024);
            rt.block_on(async {
                let producer = tokio::spawn(async move {
                    for _ in 0..MSG_COUNT {
                        tx.send(0).await.unwrap();
                    }
                });
                let consumer = tokio::spawn(async move {
                    for _ in 0..MSG_COUNT {
                        black_box(rx.recv().await.unwrap());
                    }
                });
                producer.await.unwrap();
                consumer.await.unwrap();
            });
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

    group.finish();
}

criterion_group!(benches, bench_queues);
criterion_main!(benches);
