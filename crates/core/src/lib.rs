//! translator-core: pure-Rust business core.
//!
//! Contains translation services, configuration, secrets, and shared models.
//! Has NO dependencies on UI frameworks or platform-specific APIs.

#![warn(missing_docs)]
#![deny(unsafe_code)]

pub mod audio;
pub mod config;
pub mod error;
pub mod language_direction;
pub mod model;
pub mod secrets;
pub mod service;
pub mod services;
pub mod translator;

pub use config::Config;
pub use error::{ServiceError, ServiceResult};
pub use language_direction::{resolve_direction, translate_request, TranslationDirection};
pub use model::{ServiceId, TranslateRequest, TranslateResult};
pub use service::{ApiKeyRequirement, ServiceConfig, TranslationService};
pub use translator::Translator;
