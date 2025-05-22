use criterion::{Criterion, criterion_group, criterion_main};
use safe_hook::Hook;
use safe_hook_macros::hookable;
use std::hint::black_box;
use std::sync::Arc;

#[inline(never)]
fn add(left: i32, right: i32) -> i32 {
    left + right
}

#[hookable("bench-add")]
fn add_hookable(left: i32, right: i32) -> i32 {
    left + right
}

#[inline(never)]
fn add_hookable_call(left: i32, right: i32) -> i32 {
    add_hookable(left, right)
}

#[derive(Debug)]
struct HookAdd {
    left: i32,
    right: i32,
    result: i32,
}

impl Hook for HookAdd {
    type Args<'a> = (i32, i32);
    type Result = i32;
    fn call(&self, args: (i32, i32), next: &dyn Fn((i32, i32)) -> i32) -> i32 {
        let (left, right) = args;
        let res = next((left + self.left, right + self.right));
        res + self.result
    }
}

fn benchmark(c: &mut Criterion) {
    c.bench_function("add", |b| b.iter(|| add(black_box(1), black_box(2))));
    c.bench_function("add_hookable", |b| {
        b.iter(|| add_hookable_call(black_box(1), black_box(2)))
    });
    let add_metadata = safe_hook::lookup_hookable("bench-add").unwrap();
    let hook1 = Arc::new(HookAdd {
        left: 1,
        right: 0,
        result: 0,
    });
    add_metadata.add_hook(hook1.clone()).unwrap();
    c.bench_function("add_hookable(1 hook)", |b| {
        b.iter(|| add_hookable_call(black_box(1), black_box(2)))
    });

    let hook2 = Arc::new(HookAdd {
        left: 0,
        right: 1,
        result: 0,
    });
    add_metadata.add_hook(hook2.clone()).unwrap();
    c.bench_function("add_hookable(2 hooks)", |b| {
        b.iter(|| add_hookable_call(black_box(1), black_box(2)))
    });

    let hook3 = Arc::new(HookAdd {
        left: 0,
        right: 0,
        result: 1,
    });
    add_metadata.add_hook(hook3.clone()).unwrap();
    c.bench_function("add_hookable(3 hooks)", |b| {
        b.iter(|| add_hookable_call(black_box(1), black_box(2)))
    });

    add_metadata.remove_hook(hook1.as_ref());
    add_metadata.remove_hook(hook2.as_ref());
    add_metadata.remove_hook(hook3.as_ref());
    c.bench_function("add_hookable(hooks removed)", |b| {
        b.iter(|| add_hookable_call(black_box(1), black_box(2)))
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(1000);
    targets = benchmark
);
criterion_main!(benches);
