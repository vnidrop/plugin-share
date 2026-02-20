use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc,
    Arc, Mutex, OnceLock,
};
use std::time::Duration;

use tauri::{Runtime, Window, WindowEvent};

use crate::Error;

const FOCUS_WAIT_TIMEOUT: Duration = Duration::from_secs(60);

struct FocusWaiter {
    pending: Mutex<Option<mpsc::Sender<()>>>,
    listener_registered: AtomicBool,
}

impl FocusWaiter {
    fn new() -> Self {
        Self {
            pending: Mutex::new(None),
            listener_registered: AtomicBool::new(false),
        }
    }
}

pub struct FocusWaitHandle {
    waiter: Arc<FocusWaiter>,
    rx: mpsc::Receiver<()>,
}

impl FocusWaitHandle {
    pub fn wait(self) -> Result<(), Error> {
        match self.rx.recv_timeout(FOCUS_WAIT_TIMEOUT) {
            Ok(()) => {}
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {}
        }
        clear_pending(&self.waiter);
        Ok(())
    }

    pub fn cancel(self) {
        clear_pending(&self.waiter);
    }
}

pub fn begin_focus_wait<R: Runtime>(window: &Window<R>) -> Result<FocusWaitHandle, Error> {
    let waiter = get_focus_waiter(window)?;
    ensure_focus_listener(window, waiter.clone());

    let (tx, rx) = mpsc::channel();
    {
        let mut pending = waiter
            .pending
            .lock()
            .map_err(|_| Error::NativeApi("Focus wait state poisoned.".to_string()))?;
        if pending.is_some() {
            return Err(Error::NativeApi("Share already in progress.".to_string()));
        }
        *pending = Some(tx);
    }

    Ok(FocusWaitHandle { waiter, rx })
}

fn clear_pending(waiter: &FocusWaiter) {
    if let Ok(mut pending) = waiter.pending.lock() {
        *pending = None;
    }
}

fn get_focus_waiter<R: Runtime>(window: &Window<R>) -> Result<Arc<FocusWaiter>, Error> {
    static WAITERS: OnceLock<Mutex<HashMap<String, Arc<FocusWaiter>>>> = OnceLock::new();
    let waiters = WAITERS.get_or_init(|| Mutex::new(HashMap::new()));

    let mut map = waiters
        .lock()
        .map_err(|_| Error::NativeApi("Focus wait registry poisoned.".to_string()))?;
    Ok(map
        .entry(window.label().to_string())
        .or_insert_with(|| Arc::new(FocusWaiter::new()))
        .clone())
}

fn ensure_focus_listener<R: Runtime>(window: &Window<R>, waiter: Arc<FocusWaiter>) {
    if waiter.listener_registered.swap(true, Ordering::SeqCst) {
        return;
    }

    window.on_window_event(move |event| match event {
        WindowEvent::Focused(true) | WindowEvent::Destroyed => {
            if let Ok(mut pending) = waiter.pending.lock() {
                if let Some(tx) = pending.take() {
                    let _ = tx.send(());
                }
            }
        }
        _ => {}
    });
}
