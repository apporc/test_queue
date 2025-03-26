use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use rand::{Rng, rng};
use std::thread;
use test_queue::queue::Queue;
use tokio::runtime::Runtime;

// 基准测试函数
fn bench_queues(c: &mut Criterion) {
    const MSG_COUNT: usize = 1024;

    // 测试自定义队列
    let mut group = c.benchmark_group("Queue Throughput");
    group.throughput(Throughput::Elements(MSG_COUNT as u64));
    group.measurement_time(std::time::Duration::from_secs(10));
    group.sample_size(1000);

    group.bench_function("UltraLowLatencyQueue", |b| {
        // 每次迭代创建新的队列和运行时
        b.iter(|| {
            let (mut tx, mut rx) = Queue::<u64, 1024>::new();
            let rt = Runtime::new().unwrap();

            // 生产者线程
            let producer = thread::spawn(move || {
                let mut rng = rng();
                for _ in 0..MSG_COUNT {
                    tx.send(rng.random::<u64>()).unwrap(); // 明确指定u64类型
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
                let mut rng = rng();
                for _ in 0..MSG_COUNT {
                    tx.send(rng.random::<u64>()).unwrap(); // 明确指定u64类型
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

    group.finish();
}

criterion_group!(benches, bench_queues);
criterion_main!(benches);
