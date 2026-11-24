//! Cursor/limit pagination shared by the list endpoints.
//!
//! Every list endpoint accepts an optional `limit` (1..=[`MAX_PAGE_SIZE`])
//! and an opaque `cursor` that a previous page's [`Page`] `next_cursor`
//! returned. Pages are taken over the endpoint's deterministic order
//! (jobs, keys, and webhooks are id-ordered; audit events are oldest-first).
//!
//! The cursor encodes the number of items the endpoint has already returned,
//! so it is opaque to clients: encode it with `encode_cursor` and decode it
//! with `decode_cursor` (used by [`PageRequest::parse`]).

use base64::engine::Engine as _;
use serde::Serialize;

use crate::api::{query_value, ApiError};
use crate::http::Request;

/// The page size used when `limit` is not supplied.
pub const DEFAULT_PAGE_SIZE: usize = 100;

/// The largest page size accepted; keeps even a large `limit` bounded.
pub const MAX_PAGE_SIZE: usize = 1000;

/// One page of a paginated list, with metadata for follow-up requests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Page<T> {
    /// Items on this page, in the endpoint's deterministic order.
    pub items: Vec<T>,
    /// Number of items on this page.
    pub count: usize,
    /// Whether more items exist after this page.
    pub has_more: bool,
    /// Cursor for the next page; `None` when `has_more` is false.
    pub next_cursor: Option<String>,
}

/// The parsed `limit` and `cursor` query parameters of a list request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageRequest {
    /// Number of items per page (1..=[`MAX_PAGE_SIZE`]).
    pub limit: usize,
    /// Number of items to skip, decoded from the request cursor.
    pub skip: usize,
}

impl PageRequest {
    /// Parse and validate the pagination query parameters of `request`.
    ///
    /// `limit` is an integer between 1 and [`MAX_PAGE_SIZE`] (the default is
    /// [`DEFAULT_PAGE_SIZE`]); `cursor` is the opaque cursor string returned
    /// as `next_cursor` by a previous page. Malformed values are
    /// `bad_request` errors.
    pub fn parse(request: &Request) -> Result<Self, ApiError> {
        let limit = match query_value(request, "limit") {
            Some(raw) => {
                let value = raw.trim().parse::<usize>().map_err(|_| {
                    ApiError::bad_request(format!(
                        "limit must be an integer between 1 and {MAX_PAGE_SIZE}"
                    ))
                })?;
                if value == 0 || value > MAX_PAGE_SIZE {
                    return Err(ApiError::bad_request(format!(
                        "limit must be between 1 and {MAX_PAGE_SIZE}"
                    )));
                }
                value
            }
            None => DEFAULT_PAGE_SIZE,
        };
        let skip = match query_value(request, "cursor") {
            Some(raw) if !raw.trim().is_empty() => decode_cursor(raw.trim())?,
            _ => 0,
        };
        Ok(PageRequest { limit, skip })
    }
}

/// Slice `items` (already in deterministic order) into the page that
/// follows `skip` items, bounded by `limit`.
#[must_use]
pub fn page_of<T>(items: Vec<T>, skip: usize, limit: usize) -> Page<T> {
    let total = items.len();
    let start = skip.min(total);
    let end = (start + limit).min(total);
    let items: Vec<T> = items.into_iter().skip(start).take(end - start).collect();
    let has_more = end < total;
    Page {
        next_cursor: has_more.then(|| encode_cursor(end)),
        count: items.len(),
        has_more,
        items,
    }
}

fn encode_cursor(skip: usize) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(skip.to_string())
}

fn decode_cursor(cursor: &str) -> Result<usize, ApiError> {
    let invalid = || ApiError::bad_request("cursor is not a valid pagination cursor");
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(cursor.as_bytes())
        .map_err(|_| invalid())?;
    let text = String::from_utf8(bytes).map_err(|_| invalid())?;
    text.parse::<usize>().map_err(|_| invalid())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(query: Option<&str>) -> Request {
        Request {
            method: "GET".to_string(),
            path: "/api/v1/keys".to_string(),
            query: query.map(str::to_string),
            headers: Default::default(),
            body: Vec::new(),
        }
    }

    fn items(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("item-{i}")).collect()
    }

    #[test]
    fn default_limit_and_first_page() {
        let page_request = PageRequest::parse(&request(None)).unwrap();
        assert_eq!(page_request.limit, DEFAULT_PAGE_SIZE);
        assert_eq!(page_request.skip, 0);

        let page = page_of(items(3), 0, DEFAULT_PAGE_SIZE);
        assert_eq!(page.count, 3);
        assert!(!page.has_more);
        assert!(page.next_cursor.is_none());
        assert_eq!(page.items, items(3));
    }

    #[test]
    fn pages_advance_and_has_more_matches_next_cursor() {
        let page_request = PageRequest::parse(&request(Some("limit=2"))).unwrap();
        assert_eq!(page_request.limit, 2);

        let page = page_of(items(5), 0, page_request.limit);
        assert_eq!(page.count, 2);
        assert_eq!(page.items, items(2));
        assert!(page.has_more);
        let cursor = page.next_cursor.as_deref().unwrap().to_string();

        let next_request = PageRequest::parse(&request(Some(&format!("cursor={cursor}")))).unwrap();
        assert_eq!(next_request.skip, 2);

        let page = page_of(items(5), next_request.skip, page_request.limit);
        assert_eq!(page.count, 2);
        assert!(page.has_more);

        let next_request = PageRequest::parse(&request(Some(&format!(
            "cursor={}",
            page.next_cursor.clone().unwrap()
        ))))
        .unwrap();
        assert_eq!(next_request.skip, 4);

        let page = page_of(items(5), next_request.skip, page_request.limit);
        assert_eq!(page.count, 1);
        assert!(!page.has_more);
        assert!(page.next_cursor.is_none());
    }

    #[test]
    fn cursors_are_rejected_when_malformed() {
        assert!(PageRequest::parse(&request(Some("cursor=%%%"))).is_err());
        assert!(PageRequest::parse(&request(Some("cursor=abc"))).is_err());
        assert!(PageRequest::parse(&request(Some("cursor="))).is_ok());
        assert_eq!(
            PageRequest::parse(&request(Some("cursor=NDI")))
                .unwrap()
                .skip,
            42
        );
    }

    #[test]
    fn limits_are_bounded() {
        assert!(PageRequest::parse(&request(Some("limit=0"))).is_err());
        assert!(PageRequest::parse(&request(Some("limit=-1"))).is_err());
        assert!(PageRequest::parse(&request(Some("limit=abc"))).is_err());
        assert!(PageRequest::parse(&request(Some("limit=1001"))).is_err());
        assert_eq!(
            PageRequest::parse(&request(Some("limit=1"))).unwrap().limit,
            1
        );
        assert_eq!(
            PageRequest::parse(&request(Some("limit=1000")))
                .unwrap()
                .limit,
            1000
        );
    }
}
