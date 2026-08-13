use std::num::NonZeroU32;

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
        let second = Pagination::try_from(valid(1, 100, 1)).unwrap();
        assert!(matches!(
            second.ensure_stable(Some(first)),
            Err(PaginationError::Drift)
        ));
    }
}
