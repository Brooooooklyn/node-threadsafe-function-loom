use loom::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

pub struct SimpleThreadsafeFunction {
    lock: Mutex<bool>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Status {
    Ok,
    Closed,
}

impl SimpleThreadsafeFunction {
    pub fn new() -> Self {
        SimpleThreadsafeFunction {
            lock: Mutex::new(false),
        }
    }

    pub fn call(&self) -> Status {
        let state = self.lock.lock().unwrap();
        if *state {
            Status::Closed
        } else {
            Status::Ok
        }
    }

    pub fn release(&self) {
        let mut lock = self.lock.lock().unwrap();
        *lock = true;
    }
}

pub struct HighOrderThreadsafeFunction {
    abort: Arc<AtomicBool>,
    inner: Arc<SimpleThreadsafeFunction>,
}

impl HighOrderThreadsafeFunction {
    pub fn new() -> Self {
        HighOrderThreadsafeFunction {
            abort: Arc::new(AtomicBool::new(false)),
            inner: Arc::new(SimpleThreadsafeFunction::new()),
        }
    }
    pub fn call(&self) -> Status {
        if !self.abort.load(Ordering::Acquire) {
            let status = self.inner.call();
            assert!(status == Status::Ok, "Wrong status [{:?}]", status);
            return status;
        }
        Status::Closed
    }
    pub fn abort(&self) {
        if self.abort.load(Ordering::Acquire) {
            return;
        }
        self.inner.release();
        self.abort.store(true, Ordering::Release);
    }
}

impl Clone for HighOrderThreadsafeFunction {
    fn clone(&self) -> Self {
        Self {
            abort: self.abort.clone(),
            inner: self.inner.clone(),
        }
    }
}

impl Drop for HighOrderThreadsafeFunction {
    fn drop(&mut self) {
        if self.abort.load(Ordering::Acquire) {
            return;
        }
        if Arc::strong_count(&self.inner) == 1 {
            self.inner.release();
            self.abort.store(true, Ordering::Release);
        }
    }
}

#[test]
fn test_concurrent_logic() {
    use loom::thread;

    loom::model(|| {
        let tsfn = HighOrderThreadsafeFunction::new();
        let tsfn2 = tsfn.clone();

        thread::spawn(move || {
            tsfn.call();
        });
        thread::spawn(move || {
            tsfn2.abort();
        });
    });
}
