//! Monotonic per-session sequence semantics.

use thiserror::Error;

/// Sequence assignment / validation error.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SequenceError {
    /// Sequence must be ≥ 1.
    #[error("sequence must be >= 1, got {0}")]
    NonPositive(u64),
    /// Expected next sequence value.
    #[error("sequence gap or reorder: expected {expected}, got {actual}")]
    Unexpected {
        /// Next expected sequence.
        expected: u64,
        /// Observed sequence.
        actual: u64,
    },
    /// Duplicate sequence.
    #[error("duplicate sequence {0}")]
    Duplicate(u64),
}

/// Assigns monotonic sequences starting at 1 for a single Tracer session.
#[derive(Debug, Clone, Default)]
pub struct SequenceTracker {
    next: u64,
}

impl SequenceTracker {
    /// New tracker; first `next()` returns 1.
    pub fn new() -> Self {
        Self { next: 1 }
    }

    /// Resume after last persisted sequence (so next is last+1).
    pub fn resume_after(last_sequence: u64) -> Self {
        Self {
            next: last_sequence.saturating_add(1).max(1),
        }
    }

    /// Peek at the next sequence without consuming.
    pub fn peek(&self) -> u64 {
        if self.next == 0 {
            1
        } else {
            self.next
        }
    }

    /// Allocate the next sequence number.
    pub fn next(&mut self) -> u64 {
        if self.next == 0 {
            self.next = 1;
        }
        let n = self.next;
        self.next = n.saturating_add(1);
        n
    }

    /// Last allocated sequence, or 0 if none yet.
    pub fn last_allocated(&self) -> u64 {
        self.next.saturating_sub(1)
    }
}

/// Validate that `sequences` are strictly monotonic starting at `start` (usually 1)
/// with step +1 and no duplicates/gaps.
pub fn validate_sequence_order(sequences: &[u64], start: u64) -> Result<(), SequenceError> {
    if start < 1 {
        return Err(SequenceError::NonPositive(start));
    }
    let mut expected = start;
    for &seq in sequences {
        if seq < 1 {
            return Err(SequenceError::NonPositive(seq));
        }
        if seq < expected {
            // Could be duplicate or reorder
            if sequences.iter().filter(|&&s| s == seq).count() > 1 {
                return Err(SequenceError::Duplicate(seq));
            }
            return Err(SequenceError::Unexpected {
                expected,
                actual: seq,
            });
        }
        if seq != expected {
            return Err(SequenceError::Unexpected {
                expected,
                actual: seq,
            });
        }
        expected = expected.saturating_add(1);
    }
    Ok(())
}

/// Sort envelopes' sequences and check control-plane order constraints.
pub fn sequences_are_strictly_increasing(sequences: &[u64]) -> bool {
    if sequences.is_empty() {
        return true;
    }
    sequences.windows(2).all(|w| w[0] < w[1])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracker_starts_at_one() {
        let mut t = SequenceTracker::new();
        assert_eq!(t.next(), 1);
        assert_eq!(t.next(), 2);
        assert_eq!(t.next(), 3);
        assert_eq!(t.last_allocated(), 3);
    }

    #[test]
    fn resume_after() {
        let mut t = SequenceTracker::resume_after(10);
        assert_eq!(t.next(), 11);
    }

    #[test]
    fn validate_ok() {
        assert!(validate_sequence_order(&[1, 2, 3, 4], 1).is_ok());
    }

    #[test]
    fn validate_gap() {
        let err = validate_sequence_order(&[1, 2, 4], 1).unwrap_err();
        assert_eq!(
            err,
            SequenceError::Unexpected {
                expected: 3,
                actual: 4
            }
        );
    }

    #[test]
    fn validate_reorder() {
        assert!(validate_sequence_order(&[1, 3, 2], 1).is_err());
    }

    #[test]
    fn increasing_helper() {
        assert!(sequences_are_strictly_increasing(&[1, 2, 5]));
        assert!(!sequences_are_strictly_increasing(&[1, 1]));
        assert!(!sequences_are_strictly_increasing(&[2, 1]));
    }
}
