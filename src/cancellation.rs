use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::AppError;

/// Cooperative cancellation shared by the invocation coordinator and
/// blocking filesystem workers.
#[derive(Clone, Debug, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    pub fn checkpoint(&self) -> Result<(), AppError> {
        if self.is_cancelled() {
            Err(AppError::Timeout("invocation deadline expired".into()))
        } else {
            Ok(())
        }
    }
}
