use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc,
    Arc, Mutex, OnceLock,
};
use std::time::Duration;

use tauri::{Runtime, Window, WindowEvent};

use crate::Error;

const FOCUS_WAIT_GRACE: Duration = Duration::from_millis(250);
const FOCUS_WAIT_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum FocusPhase {
    WaitingForLoss,
    WaitingForRegain,
    Completed,
}

#[derive(Debug)]
struct FocusState {
    phase: FocusPhase,
}

impl FocusState {
    fn new() -> Self {
        Self {
            phase: FocusPhase::WaitingForLoss,
        }
    }

    fn reset(&mut self) {
        self.phase = FocusPhase::WaitingForLoss;
    }

    fn mark_completed(&mut self) {
        self.phase = FocusPhase::Completed;
    }

    fn on_focus_change(&mut self, focused: bool) -> bool {
        match (self.phase, focused) {
            (FocusPhase::WaitingForLoss, false) => {
                self.phase = FocusPhase::WaitingForRegain;
                false
            }
            (FocusPhase::WaitingForRegain, true) => {
                self.phase = FocusPhase::Completed;
                true
            }
            _ => false,
        }
    }

    fn on_destroyed(&mut self) -> bool {
        self.phase = FocusPhase::Completed;
        true
    }

    fn on_grace_elapsed(&mut self) -> bool {
        if self.phase == FocusPhase::WaitingForLoss {
            self.phase = FocusPhase::Completed;
            return true;
        }
        false
    }
}

struct FocusWaiter {
    label: String,
    pending: Mutex<Option<mpsc::Sender<()>>>,
    state: Mutex<FocusState>,
    listener_registered: AtomicBool,
}

impl FocusWaiter {
    fn new(label: String) -> Self {
        Self {
            label,
            pending: Mutex::new(None),
            state: Mutex::new(FocusState::new()),
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
    if let Ok(mut state) = waiter.state.lock() {
        state.reset();
    }

    spawn_focus_grace_timer(waiter.clone());

    Ok(FocusWaitHandle { waiter, rx })
}

fn clear_pending(waiter: &FocusWaiter) {
    if let Ok(mut pending) = waiter.pending.lock() {
        *pending = None;
    }
    if let Ok(mut state) = waiter.state.lock() {
        state.mark_completed();
    }
}

fn get_focus_waiter<R: Runtime>(window: &Window<R>) -> Result<Arc<FocusWaiter>, Error> {
    let mut map = focus_waiters()
        .lock()
        .map_err(|_| Error::NativeApi("Focus wait registry poisoned.".to_string()))?;
    Ok(map
        .entry(window.label().to_string())
        .or_insert_with(|| Arc::new(FocusWaiter::new(window.label().to_string())))
        .clone())
}

fn ensure_focus_listener<R: Runtime>(window: &Window<R>, waiter: Arc<FocusWaiter>) {
    if waiter.listener_registered.swap(true, Ordering::SeqCst) {
        return;
    }

    window.on_window_event(move |event| match event {
        WindowEvent::Focused(focused) => {
            if should_complete_on_focus(&waiter, *focused) {
                complete_wait(&waiter);
            }
        }
        WindowEvent::Destroyed => {
            if should_complete_on_destroy(&waiter) {
                complete_wait(&waiter);
            }
            remove_focus_waiter(&waiter.label);
        }
        _ => {}
    });
}

fn should_complete_on_focus(waiter: &FocusWaiter, focused: bool) -> bool {
    let mut state = match waiter.state.lock() {
        Ok(state) => state,
        Err(poisoned) => poisoned.into_inner(),
    };
    state.on_focus_change(focused)
}

fn should_complete_on_destroy(waiter: &FocusWaiter) -> bool {
    let mut state = match waiter.state.lock() {
        Ok(state) => state,
        Err(poisoned) => poisoned.into_inner(),
    };
    state.on_destroyed()
}

fn spawn_focus_grace_timer(waiter: Arc<FocusWaiter>) {
    std::thread::spawn(move || {
        std::thread::sleep(FOCUS_WAIT_GRACE);
        let should_complete = {
            let mut state = match waiter.state.lock() {
                Ok(state) => state,
                Err(poisoned) => poisoned.into_inner(),
            };
            state.on_grace_elapsed()
        };
        if should_complete {
            complete_wait(&waiter);
        }
    });
}

fn complete_wait(waiter: &FocusWaiter) {
    let sender = match waiter.pending.lock() {
        Ok(mut pending) => pending.take(),
        Err(poisoned) => poisoned.into_inner().take(),
    };
    if let Some(tx) = sender {
        let _ = tx.send(());
    }
    if let Ok(mut state) = waiter.state.lock() {
        state.mark_completed();
    }
}

fn remove_focus_waiter(label: &str) {
    if let Ok(mut map) = focus_waiters().lock() {
        map.remove(label);
    }
}

fn focus_waiters() -> &'static Mutex<HashMap<String, Arc<FocusWaiter>>> {
    static WAITERS: OnceLock<Mutex<HashMap<String, Arc<FocusWaiter>>>> = OnceLock::new();
    WAITERS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(test)]
mod tests {
    use super::{FocusPhase, FocusState};

    #[test]
    fn focus_state_completes_after_loss_and_regain() {
        let mut state = FocusState::new();
        assert!(!state.on_focus_change(true));
        assert!(!state.on_focus_change(false));
        assert!(!state.on_focus_change(false));
        assert!(state.on_focus_change(true));
        assert_eq!(state.phase, FocusPhase::Completed);
    }

    #[test]
    fn focus_state_completes_on_destroy() {
        let mut state = FocusState::new();
        assert!(state.on_destroyed());
        assert_eq!(state.phase, FocusPhase::Completed);
    }

    #[test]
    fn focus_state_grace_completes_without_focus_loss() {
        let mut state = FocusState::new();
        assert!(state.on_grace_elapsed());
        assert_eq!(state.phase, FocusPhase::Completed);
    }

    #[test]
    fn focus_state_grace_does_not_complete_after_loss() {
        let mut state = FocusState::new();
        assert!(!state.on_focus_change(false));
        assert!(!state.on_grace_elapsed());
        assert_eq!(state.phase, FocusPhase::WaitingForRegain);
    }
}
