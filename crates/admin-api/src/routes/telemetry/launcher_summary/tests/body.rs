//! The 64 KiB body cap. The handler reads the body itself, after the kill
//! switch and the quota, and stops at the cap; `order` pins where that sits
//! among the refusals and `quota` that an oversized request is charged.

use super::super::dto::Verdict::Accepted;
use super::super::MAX_SUMMARY_BODY_BYTES;
use super::{batch, capture, element, refusal, summary_rows, Env, Harness};

const TOO_LARGE: &str = "Body is over 64 KiB";

/// A valid one-element request padded with trailing spaces to `len` bytes.
/// JSON allows the whitespace, so the padding changes the size and nothing
/// else.
fn padded(n: u32, len: usize) -> Vec<u8> {
    let mut body = batch(vec![element(n)]).to_string().into_bytes();
    assert!(body.len() <= len);
    body.resize(len, b' ');
    body
}

/// **The cap is 64 KiB to the byte.** A valid request of exactly 65,536
/// bytes is read whole and its element accepted; the same request one
/// space longer is a 413 with a static body that writes no row and
/// remembers no id, so its element is accepted as new at the legal size.
#[test]
fn a_body_of_exactly_64_kib_is_read_and_one_byte_more_is_a_413() {
    let _env = Env::install();
    assert_eq!(MAX_SUMMARY_BODY_BYTES, 65_536);
    let h = Harness::new();

    let (result, rows) = capture(|| h.post(&padded(1, MAX_SUMMARY_BODY_BYTES)));
    assert_eq!(result.expect("64 KiB").results, [Accepted]);
    assert_eq!(summary_rows(&rows).len(), 1);

    let (result, rows) = capture(|| h.post(&padded(2, MAX_SUMMARY_BODY_BYTES + 1)));
    let r = refusal(result.expect_err("64 KiB + 1"));
    assert_eq!(r.status, 413, "{r:?}");
    assert_eq!(r.body, TOO_LARGE);
    assert_eq!(r.retry_after, None);
    assert!(rows.is_empty(), "{rows:#?}");

    let at_cap = h.post(&padded(2, MAX_SUMMARY_BODY_BYTES));
    assert_eq!(at_cap.expect("control").results, [Accepted]);
}

/// A body far over the cap is the same static 413: nothing of it is parsed,
/// so it does not matter that this one would have been a valid request.
#[test]
fn a_body_far_over_the_cap_is_a_413() {
    let _env = Env::install();
    let h = Harness::new();
    let (result, rows) = capture(|| h.post(&padded(1, 4 * 1024 * 1024)));
    let r = refusal(result.expect_err("4 MiB"));
    assert_eq!((r.status, r.body.as_str()), (413, TOO_LARGE));
    assert!(rows.is_empty(), "{rows:#?}");
}
