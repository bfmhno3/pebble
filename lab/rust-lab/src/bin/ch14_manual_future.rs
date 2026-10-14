use std::future::Future;
use std::pin::{Pin, pin};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};
use std::thread::{self, Thread};
use std::time::{Duration, Instant};

/// A future that finishes after `remaining` polls and asks to be polled again each time.
struct CountDown {
    remaining: u32,
}

impl Future for CountDown {
    type Output = &'static str;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<&'static str> {
        if self.remaining == 0 {
            Poll::Ready("liftoff")
        } else {
            println!("  CountDown::poll, {} left", self.remaining);
            self.remaining -= 1;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

/// State shared between a `Delay` and the thread that completes it.
struct Shared {
    done: bool,
    waker: Option<Waker>,
}

/// A future that completes after a duration, driven by a helper thread.
struct Delay {
    shared: Arc<Mutex<Shared>>,
}

impl Delay {
    fn new(duration: Duration) -> Self {
        let shared = Arc::new(Mutex::new(Shared {
            done: false,
            waker: None,
        }));
        let thread_shared = Arc::clone(&shared);
        thread::spawn(move || {
            thread::sleep(duration);
            let mut state = thread_shared.lock().unwrap();
            state.done = true;
            if let Some(waker) = state.waker.take() {
                waker.wake();
            }
        });
        Delay { shared }
    }
}

impl Future for Delay {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let mut state = self.shared.lock().unwrap();
        if state.done {
            Poll::Ready(())
        } else {
            state.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

/// Wakes a parked thread.
struct ThreadWaker(Thread);

impl Wake for ThreadWaker {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

static POLLS: AtomicUsize = AtomicUsize::new(0);

/// Runs a future to completion on the current thread.
fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let waker = Waker::from(Arc::new(ThreadWaker(thread::current())));
    let mut cx = Context::from_waker(&waker);
    loop {
        POLLS.fetch_add(1, Ordering::Relaxed);
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(value) => return value,
            Poll::Pending => thread::park(),
        }
    }
}

async fn two_delays() -> u32 {
    println!("  two_delays starts");
    Delay::new(Duration::from_millis(50)).await;
    println!("  first delay done");
    Delay::new(Duration::from_millis(50)).await;
    println!("  second delay done");
    42
}

fn main() {
    println!("countdown:");
    let message = block_on(CountDown { remaining: 3 });
    println!("{message} after {} polls", POLLS.swap(0, Ordering::Relaxed));

    println!("two delays:");
    let start = Instant::now();
    let answer = block_on(two_delays());
    let elapsed = start.elapsed();
    println!("answer {answer} after {} polls", POLLS.swap(0, Ordering::Relaxed));
    println!("took at least 100 ms: {}", elapsed >= Duration::from_millis(100));

    println!("size of the async fn future: {} bytes", std::mem::size_of_val(&two_delays()));
}
