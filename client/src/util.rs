use std::sync::atomic::{AtomicUsize, Ordering};

pub fn get_uid() -> usize {
    static UID: AtomicUsize = AtomicUsize::new(1);
    UID.fetch_add(1, Ordering::Relaxed)
}
