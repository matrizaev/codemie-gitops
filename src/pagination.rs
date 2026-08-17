use std::num::NonZeroU32;

use crate::error::AppError;

/// Shared server-list traversal budget.
pub(crate) const SERVER_LIST_MAX_ITEMS: u32 = 100_000;
const SERVER_LIST_MAX_PAGES: u32 = 1_000;
pub(crate) const SERVER_LIST_PAGE_SIZE: u32 = 100;

/// Validated, stable pagination metadata for one server-list page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Pagination {
    pages: u32,
    total: u32,
    per_page: NonZeroU32,
}

/// Untrusted pagination metadata from an API response.
pub(crate) struct PaginationInput {
    pub(crate) requested_page: u32,
    pub(crate) page: u32,
    pub(crate) per_page: u32,
    pub(crate) total: u32,
    pub(crate) pages: u32,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum PaginationError {
    #[error("pagination origin changed")]
    Origin,
    #[error("pagination page size changed")]
    PageSize,
    #[error("pagination exceeded the page cap")]
    PageCap,
    #[error("zero page count does not match zero total")]
    ZeroMismatch,
    #[error("pagination total and page count disagree")]
    PageCount,
    #[error("pagination changed during enumeration")]
    Drift,
}

impl PaginationError {
    pub(crate) fn is_drift(&self) -> bool {
        matches!(self, Self::Drift)
    }
}

impl TryFrom<PaginationInput> for Pagination {
    type Error = PaginationError;

    fn try_from(input: PaginationInput) -> Result<Self, Self::Error> {
        if input.page != input.requested_page {
            return Err(PaginationError::Origin);
        }
        if input.per_page != SERVER_LIST_PAGE_SIZE {
            return Err(PaginationError::PageSize);
        }
        if input.pages > SERVER_LIST_MAX_PAGES {
            return Err(PaginationError::PageCap);
        }
        // A non-empty listing only has indices 0..pages-1; a response that
        // reports a requested page at or beyond its own page count is
        // incompatible (defense in depth: consumers also enforce the origin).
        if input.pages != 0 && input.page >= input.pages {
            return Err(PaginationError::PageCap);
        }
        if (input.pages == 0) != (input.total == 0) {
            return Err(PaginationError::ZeroMismatch);
        }
        let per_page = NonZeroU32::new(input.per_page).ok_or(PaginationError::PageSize)?;
        if input.pages != input.total.div_ceil(per_page.get()) {
            return Err(PaginationError::PageCount);
        }
        Ok(Self {
            pages: input.pages,
            total: input.total,
            per_page,
        })
    }
}

impl Pagination {
    pub(crate) fn ensure_stable(self, previous: Option<Self>) -> Result<Self, PaginationError> {
        if previous.is_some_and(|previous| previous != self) {
            return Err(PaginationError::Drift);
        }
        Ok(self)
    }

    pub(crate) fn pages(self) -> u32 {
        self.pages
    }

    pub(crate) fn total(self) -> u32 {
        self.total
    }
}

// ---------------------------------------------------------------------------
// Shared exhaustive page scan
// ---------------------------------------------------------------------------

/// One decoded page supplied to `scan_pages` by the entity adapter.
pub(crate) struct Page<T> {
    pub(crate) items: Vec<T>,
    pub(crate) pagination: PaginationInput,
}

/// Items whose server id is exposed for duplicate/cap detection.
pub(crate) trait PageItem {
    fn page_item_id(&self) -> &str;
}

/// Result of a completed exhaustive scan.
pub(crate) struct Scan<T> {
    pub(crate) items: Vec<T>,
    pub(crate) pages_requested: u32,
    pub(crate) items_seen: u32,
    pub(crate) advertised_total: u32,
}

/// Shared zero-based exhaustive scan used by every kind adapter.
///
/// Enforces the shared traversal invariants in one place: page 0 first, exact
/// page-size 100, page-range/origin validation, fingerprint stability across
/// pages (drift/cycles), the 100,000-item cap, repeated-id detection, and
/// advertised-total reconciliation. `fetch_page(page)` decodes and
/// validates one page; item-level filtering stays with the caller after the
/// scan.
pub(crate) async fn scan_pages<T, F, Fut>(
    entity: &'static str,
    mut fetch_page: F,
) -> Result<Scan<T>, AppError>
where
    T: PageItem,
    F: FnMut(u32) -> Fut,
    Fut: std::future::Future<Output = Result<Page<T>, AppError>>,
{
    let mut all = Vec::new();
    let mut page = 0u32;
    let mut pages_requested = 0u32;
    let mut total_seen = 0u32;
    let mut fingerprint: Option<Pagination> = None;
    let mut seen_ids = std::collections::HashSet::new();

    loop {
        let Page {
            items,
            pagination: input,
        } = fetch_page(page).await?;
        pages_requested += 1;
        let pagination = Pagination::try_from(input)
            .and_then(|validated| validated.ensure_stable(fingerprint))
            .map_err(|error| {
                if error.is_drift() {
                    AppError::Reconciliation(format!("{entity} {error}"))
                } else {
                    AppError::ApiIncompatible(format!("{entity} {error}"))
                }
            })?;
        fingerprint = Some(pagination);

        for item in items {
            total_seen += 1;
            if total_seen > SERVER_LIST_MAX_ITEMS {
                return Err(AppError::ApiIncompatible(format!(
                    "{entity} enumeration exceeded 100,000-item cap"
                )));
            }
            if !seen_ids.insert(item.page_item_id().to_owned()) {
                return Err(AppError::Reconciliation(format!(
                    "{entity} enumeration repeated an entity id"
                )));
            }
            all.push(item);
        }

        if pagination.pages() == 0 || page + 1 >= pagination.pages() {
            break;
        }
        page += 1;
    }

    let advertised_total = fingerprint.map_or(0, Pagination::total);
    if total_seen != advertised_total {
        return Err(AppError::Reconciliation(format!(
            "{entity} enumeration ended before the advertised total"
        )));
    }
    Ok(Scan {
        items: all,
        pages_requested,
        items_seen: total_seen,
        advertised_total,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid(page: u32, total: u32, pages: u32) -> PaginationInput {
        PaginationInput {
            requested_page: page,
            page,
            per_page: SERVER_LIST_PAGE_SIZE,
            total,
            pages,
        }
    }

    #[test]
    fn accepts_consistent_zero_indexed_metadata() {
        let pagination = Pagination::try_from(valid(1, 101, 2)).unwrap();
        assert_eq!(pagination.pages(), 2);
        assert_eq!(pagination.total(), 101);
    }

    #[test]
    fn rejects_every_shared_invariant_violation() {
        let cases = [
            PaginationInput {
                requested_page: 0,
                page: 1,
                per_page: 100,
                total: 0,
                pages: 0,
            },
            PaginationInput {
                requested_page: 0,
                page: 0,
                per_page: 99,
                total: 0,
                pages: 0,
            },
            valid(0, 100_001, 1_001),
            valid(0, 0, 1),
            valid(0, 101, 1),
        ];
        assert!(
            cases
                .into_iter()
                .all(|case| Pagination::try_from(case).is_err())
        );
    }

    #[test]
    fn detects_fingerprint_drift() {
        let first = Pagination::try_from(valid(0, 101, 2)).unwrap();
        let second = Pagination::try_from(valid(1, 200, 2)).unwrap();
        assert!(matches!(
            second.ensure_stable(Some(first)),
            Err(PaginationError::Drift)
        ));
    }
}
