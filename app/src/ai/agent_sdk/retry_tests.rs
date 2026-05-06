use std::cell::Cell;
use std::rc::Rc;

use anyhow::anyhow;
use futures::executor::block_on;

use super::*;

#[test]
fn errors_without_http_status_are_treated_as_transient() {
    let err = anyhow!("connection reset by peer");
    assert!(is_transient_http_error(&err));

    let err = anyhow!("Failed to send request: timed out");
    assert!(is_transient_http_error(&err));
}

#[test]
fn retry_loop_succeeds_on_first_attempt() {
    let attempts = Rc::new(Cell::new(0));
    let attempts_clone = attempts.clone();
    let result: Result<()> = block_on(with_bounded_retry("test retry", || {
        attempts_clone.set(attempts_clone.get() + 1);
        async { Ok(()) }
    }));
    result.unwrap();
    assert_eq!(attempts.get(), 1);
}

#[test]
fn retry_loop_retries_transient_and_eventually_succeeds() {
    let attempts = Rc::new(Cell::new(0));
    let attempts_clone = attempts.clone();
    let result: Result<u32> = block_on(with_bounded_retry("test retry", || {
        let n = attempts_clone.get() + 1;
        attempts_clone.set(n);
        async move {
            if n < 2 {
                Err(anyhow!("transient failure"))
            } else {
                Ok(n)
            }
        }
    }));
    assert_eq!(result.unwrap(), 2);
    assert_eq!(attempts.get(), 2);
}

#[test]
fn retry_loop_stops_at_max_attempts_on_persistent_transient() {
    let attempts = Rc::new(Cell::new(0));
    let attempts_clone = attempts.clone();
    let result: Result<()> = block_on(with_bounded_retry("test retry", || {
        attempts_clone.set(attempts_clone.get() + 1);
        async { Err(anyhow!("transient failure")) }
    }));
    assert!(result.is_err());
    assert_eq!(attempts.get(), MAX_ATTEMPTS);
}
