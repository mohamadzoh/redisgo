use criterion::{criterion_group, criterion_main, Criterion};
use redisgo::RedisGo;
use std::time::Duration;

const SAMPLE_SIZE: usize = 100;
const MEASUREMENT_TIME: Duration = Duration::from_secs(120);
const WARM_UP_TIME: Duration = Duration::from_secs(30);

fn benchmark_set(c: &mut Criterion) {
    let mut group = c.benchmark_group("redisgo::set");
    group.sample_size(SAMPLE_SIZE);
    group.measurement_time(MEASUREMENT_TIME);
    group.warm_up_time(WARM_UP_TIME);
    group.bench_function("set", |b| {
        b.iter(|| {
            RedisGo::set("bench_key", "bench_value").expect("Failed to set cache");
        });
    });
    group.finish();
}

fn benchmark_get(c: &mut Criterion) {
    let mut group = c.benchmark_group("redisgo::get");
    group.sample_size(SAMPLE_SIZE);
    group.measurement_time(MEASUREMENT_TIME);
    group.warm_up_time(WARM_UP_TIME);
    group.bench_function("get", |b| {
        b.iter(|| {
            let _ = RedisGo::get("bench_key").expect("Failed to get cache");
        });
    });
    group.finish();
}

fn benchmark_delete(c: &mut Criterion) {
    let mut group = c.benchmark_group("redisgo::delete");
    group.sample_size(SAMPLE_SIZE);
    group.measurement_time(MEASUREMENT_TIME);
    group.warm_up_time(WARM_UP_TIME);
    group.bench_function("delete", |b| {
        b.iter(|| {
            RedisGo::delete("bench_key").expect("Failed to delete cache");
        });
    });
    group.finish();
}

fn benchmark_exists(c: &mut Criterion) {
    let mut group = c.benchmark_group("redisgo::exists");
    group.sample_size(SAMPLE_SIZE);
    group.measurement_time(MEASUREMENT_TIME);
    group.warm_up_time(WARM_UP_TIME);
    group.bench_function("exists", |b| {
        b.iter(|| {
            let _ = RedisGo::exists("bench_key").expect("Failed to check existence");
        });
    });
    group.finish();
}

fn benchmark_flush_all(c: &mut Criterion) {
    let mut group = c.benchmark_group("redisgo::flush_all");
    group.sample_size(SAMPLE_SIZE);
    group.measurement_time(MEASUREMENT_TIME);
    group.warm_up_time(WARM_UP_TIME);
    group.bench_function("flush_all", |b| {
        b.iter(|| {
            RedisGo::flush_all().expect("Failed to flush all caches");
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    benchmark_set,
    benchmark_get,
    benchmark_delete,
    benchmark_exists,
    benchmark_flush_all
);
criterion_main!(benches);
