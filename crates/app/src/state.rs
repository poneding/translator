//! Shared application state held by the Tauri runtime.

use std::sync::Arc;

use tokio::sync::Mutex;

use translator_core::Translator;

/// State accessible to all `#[tauri::command]` handlers.
pub struct AppState {
    /// The translation coordinator (parallel service dispatch).
    pub translator: Arc<Translator>,
    /// Lazily-initialized platform selection monitor.
    pub selection: Mutex<Option<Arc<dyn translator_platform::SelectionMonitor>>>,
}

impl AppState {
    /// Construct a new state.
    pub fn new() -> Self {
        Self {
            translator: Arc::new(Translator::new()),
            selection: Mutex::new(None),
        }
    }

    /// Get (or lazily construct) the selection monitor for the current platform.
    pub async fn selection_monitor(&self) -> Arc<dyn translator_platform::SelectionMonitor> {
        let mut guard = self.selection.lock().await;
        if let Some(m) = guard.as_ref() {
            return m.clone();
        }
        let m: Arc<dyn translator_platform::SelectionMonitor> =
            Arc::from(translator_platform::create());
        *guard = Some(m.clone());
        m
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
