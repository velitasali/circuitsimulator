//! Shared paint queue for one instrument window.
//!
//! The GUI thread stores the size and a snapshot. One worker paints it.
//! Each window has its own `Host`, so the scope and the logic analyzer do not
//! share a queue, a thread, or an `image_frame` slot.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex, Once};
use std::thread;

use cs_engine::canvas::ImageBuf;

use crate::image_frame::{self, Slot, View};

#[derive(Clone, Copy)]
struct FrameSize {
    w: f64,
    h: f64,
    dpr: f64,
}

/// Device-pixel size of an instrument window. The worker and the cursor query
/// both use this so a marker lands on the raster.
pub(super) fn device_size(w: f64, h: f64, dpr: f64) -> (f64, f64, f64) {
    let dpr = if dpr.is_finite() && dpr > 0.1 {
        dpr
    } else {
        1.0
    };
    let pw = ((w * dpr).round() as u32).clamp(1, 4096) as f64;
    let ph = ((h * dpr).round() as u32).clamp(1, 4096) as f64;
    (pw, ph, dpr)
}

pub struct Frame<P> {
    pub epoch: u64,
    pub w: f64,
    pub h: f64,
    pub dpr: f64,
    pub pw: u32,
    pub ph: u32,
    pub payload: P,
}

struct Shared<P> {
    pending: Option<Frame<P>>,
    size: Option<FrameSize>,
    /// Bumped when the item is destroyed. An in-flight job with another epoch is dropped.
    epoch: u64,
}

/// Address of a process-lifetime `Host`. Sent to the worker thread.
struct WorkerRef<P: Send + 'static>(*const Host<P>);

// The pointed-to `Host` is a static. The worker and the GUI thread share it
// through the mutex and the condvar.
unsafe impl<P: Send + 'static> Send for WorkerRef<P> {}

impl<P: Send + 'static> WorkerRef<P> {
    fn run(self) {
        // SAFETY: `self.0` points at a static `Host`, which is never moved.
        unsafe { (*self.0).worker_loop() }
    }
}

pub struct Host<P: Send + 'static> {
    shared: Mutex<Shared<P>>,
    cv: Condvar,
    flush_posted: AtomicBool,
    started: Once,
    paint: fn(&Frame<P>) -> Option<ImageBuf>,
    slot: Slot,
    name: &'static str,
}

impl<P: Send + 'static> Host<P> {
    pub const fn new(
        name: &'static str,
        slot: Slot,
        paint: fn(&Frame<P>) -> Option<ImageBuf>,
    ) -> Self {
        Self {
            shared: Mutex::new(Shared {
                pending: None,
                size: None,
                epoch: 0,
            }),
            cv: Condvar::new(),
            flush_posted: AtomicBool::new(false),
            started: Once::new(),
            paint,
            slot,
            name,
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Shared<P>> {
        self.shared.lock().unwrap_or_else(|err| err.into_inner())
    }

    /// Remember the window size and paint. A closed or tiny window is left unchanged.
    pub fn render(&'static self, w: f64, h: f64, dpr: f64, snapshot: impl FnOnce() -> P) {
        if w < 2.0 || h < 2.0 {
            return;
        }
        {
            let mut shared = self.lock();
            shared.size = Some(FrameSize { w, h, dpr });
        }
        self.enqueue(snapshot);
    }

    /// Queue one paint for the current window. Returns false when no window is open
    /// or a flush is already waiting, so a burst of setters posts a single event.
    pub fn schedule(&'static self) -> bool {
        if self.lock().size.is_none() {
            return false;
        }
        self.flush_posted
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    pub fn cancel_flush(&'static self) {
        self.flush_posted.store(false, Ordering::Release);
    }

    /// Snapshot after the current Qt event. Setters in that event share this call.
    pub fn flush(&'static self, snapshot: impl FnOnce() -> P) {
        self.flush_posted.store(false, Ordering::Release);
        self.enqueue(snapshot);
    }

    /// The item is gone. Drop a queued job and ignore the one already rendering.
    pub fn close(&'static self) {
        let mut shared = self.lock();
        shared.epoch = shared.epoch.wrapping_add(1);
        shared.pending = None;
        shared.size = None;
        self.flush_posted.store(false, Ordering::Release);
    }

    fn enqueue(&'static self, snapshot: impl FnOnce() -> P) {
        if self.lock().size.is_none() {
            return;
        }
        let payload = snapshot();
        {
            let mut shared = self.lock();
            let Some(size) = shared.size else {
                return;
            };
            let (pw, ph, dpr) = device_size(size.w, size.h, size.dpr);
            shared.pending = Some(Frame {
                epoch: shared.epoch,
                w: size.w,
                h: size.h,
                dpr,
                pw: pw as u32,
                ph: ph as u32,
                payload,
            });
        }
        self.cv.notify_one();
        self.ensure_worker();
    }

    fn ensure_worker(&'static self) {
        // Copy the address before `call_once` borrows `started`.
        let host = WorkerRef(self as *const Host<P>);
        let name = self.name;
        self.started.call_once(|| {
            let _ = thread::Builder::new()
                .name(name.into())
                .spawn(move || host.run());
        });
    }

    fn worker_loop(&self) {
        loop {
            let job = {
                let mut shared = self.lock();
                loop {
                    if let Some(job) = shared.pending.take() {
                        break job;
                    }
                    shared = self.cv.wait(shared).unwrap_or_else(|err| err.into_inner());
                }
            };
            let image = (self.paint)(&job);
            let shared = self.lock();
            if shared.epoch != job.epoch || shared.pending.is_some() {
                continue;
            }
            if let Some(image) = image {
                image_frame::publish(self.slot, image, View::at(job.w, job.h, job.dpr));
            }
        }
    }
}
