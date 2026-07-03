use criterion::{Criterion, criterion_group, criterion_main};

pub struct Inner<R> {
    pub data: R,
    pub delta: f64,
}

pub struct TestRefCell<R> {
    inner: std::rc::Rc<std::cell::RefCell<Inner<R>>>,
}

impl<R> TestRefCell<R> {
    pub fn get(&self) {}

    pub fn get_mut<O>(&mut self, cb: impl FnOnce(&mut Inner<R>) -> O) -> O {
        let mut r = self.inner.borrow_mut();
        cb(&mut *r)
    }
}

pub struct TestUnsafeCell<R> {
    inner: std::rc::Rc<std::cell::UnsafeCell<Inner<R>>>,
}

impl<R> TestUnsafeCell<R> {
    pub fn get_mut<O>(&mut self, cb: impl FnOnce(&mut Inner<R>) -> O) -> O {
        let r = unsafe { &mut *(self.inner.get()) };
        cb(r)
    }
}

pub struct TestPointer<R> {
    inner: *mut Inner<R>,
}

impl<R> TestPointer<R> {
    pub fn get_mut<O>(&mut self, cb: impl FnOnce(&mut Inner<R>) -> O) -> O {
        let r = unsafe { &mut *self.inner };
        cb(r)
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    //
    let test_refcell = TestRefCell {
        inner: std::rc::Rc::new(std::cell::RefCell::new(Inner {
            data: "hello".to_string(),
            delta: 10.0,
        })),
    };
    c.bench_function("test_refcell clone", |b| {
        b.iter(|| {
            let _r = std::hint::black_box(TestRefCell {
                inner: test_refcell.inner.clone(),
            });
        });
    });

    //
    let test_unsafecell = TestUnsafeCell {
        inner: std::rc::Rc::new(std::cell::UnsafeCell::new(Inner {
            data: "hello".to_string(),
            delta: 10.0,
        })),
    };
    c.bench_function("test_unsafecell clone", |b| {
        b.iter(|| {
            let _r = std::hint::black_box(TestUnsafeCell {
                inner: test_unsafecell.inner.clone(),
            });
        });
    });

    //
    let test_pointer = TestPointer {
        inner: test_unsafecell.inner.get(),
    };
    c.bench_function("test_pointer copy", |b| {
        b.iter(|| {
            let _r = std::hint::black_box(TestPointer {
                inner: test_pointer.inner,
            });
        });
    });

    //
    c.bench_function("test_refcell borrow", |b| {
        b.iter_batched(
            || {
                //
                TestRefCell {
                    inner: std::rc::Rc::new(std::cell::RefCell::new(Inner {
                        data: "hello".to_string(),
                        delta: 10.0,
                    })),
                }
            },
            |mut input| {
                //
                std::hint::black_box(input.get_mut(|inner| {
                    inner.data.replace_range(0..1, "y");
                    assert_eq!(inner.data, "yello");
                }));
            },
            criterion::BatchSize::SmallInput, // Or LargeInput based on your data
        )
    });

    //
    c.bench_function("test_unsafecell borrow", |b| {
        b.iter_batched(
            || {
                //
                TestUnsafeCell {
                    inner: std::rc::Rc::new(std::cell::UnsafeCell::new(Inner {
                        data: "hello".to_string(),
                        delta: 10.0,
                    })),
                }
            },
            |mut input| {
                //
                std::hint::black_box(input.get_mut(|inner| {
                    inner.data.replace_range(0..1, "y");
                    assert_eq!(inner.data, "yello");
                }));
            },
            criterion::BatchSize::SmallInput, // Or LargeInput based on your data
        )
    });

    //
    c.bench_function("test_pointer borrow", |b| {
        b.iter_batched(
            || {
                //
                let test_unsafecell = TestUnsafeCell {
                    inner: std::rc::Rc::new(std::cell::UnsafeCell::new(Inner {
                        data: "hello".to_string(),
                        delta: 10.0,
                    })),
                };
                TestPointer {
                    inner: test_unsafecell.inner.get(),
                }
            },
            |mut input| {
                //
                std::hint::black_box(input.get_mut(|inner| {
                    inner.data.replace_range(0..1, "y");
                    assert_eq!(inner.data, "yello");
                }));
            },
            criterion::BatchSize::SmallInput, // Or LargeInput based on your data
        )
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
