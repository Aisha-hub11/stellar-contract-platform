//! Versioned REST API for the Stellar Contract Observatory.
//!
//! The API is a transport adapter over `observatory-platform`. It contains no
//! analysis logic; it validates requests, applies authentication, authorization,
//! and rate limiting, and serializes versioned report envelopes.
//!
//! - [`api::Api`] is the transport-agnostic router.
//! - [`http`] is a minimal bounded HTTP/1.1 adapter for local/test use.
//! - [`metrics`] renders process-local counters in Prometheus text format.
//! - [`openapi`] generates and checks the OpenAPI document.

pub mod api;
pub mod dto;
pub mod errors;
pub mod http;
pub mod metrics;
pub mod openapi;
pub mod pagination;

pub use api::{Api, ApiError, API_PREFIX, METRICS_PATH, ROUTES};
pub use dto::{CompatibilityRequest, DiffRequest, InspectRequest, WasmRequest};
pub use errors::{entry as error_catalog_entry, ErrorCatalogEntry, ERROR_CATALOG};
pub use http::{HttpLimits, Request, Response};
pub use metrics::{Metrics, MetricsSnapshot};
pub use pagination::{Page, PageRequest, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE};
