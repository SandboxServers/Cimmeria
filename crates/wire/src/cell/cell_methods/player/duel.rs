//! `sendDuelResponse(INT8 aResponse)`, SGWPlayer cell method 102
//! (`SGWPlayer.def:975-978`), and its decoder.
//!
//! The client's challenge prompt is a generic Yes/No whose Yes calls
//! `duelResponse(true)` (`Content/UI/Core/Duel/Duel.lua`, audit A-50), so
//! 1 is accept and 0 is decline. Any other value is refused rather than
//! read as a yes.

/// The caller's answer to a pending challenge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DuelResponse {
    Decline,
    Accept,
}

impl DuelResponse {
    /// Stable value for the `response` log field.
    pub fn name(self) -> &'static str {
        match self {
            DuelResponse::Decline => "decline",
            DuelResponse::Accept => "accept",
        }
    }
}

/// Why a `sendDuelResponse` payload did not decode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DuelResponseDecodeError {
    /// Not exactly one byte.
    BadLength(usize),
    /// A byte other than 0 or 1.
    UnknownValue(i8),
}

impl DuelResponseDecodeError {
    /// Stable value for the `reason` log field.
    pub fn reason(self) -> &'static str {
        match self {
            DuelResponseDecodeError::BadLength(_) => "bad_length",
            DuelResponseDecodeError::UnknownValue(_) => "unknown_response",
        }
    }
}

/// Decode the one `INT8` argument.
pub fn decode_send_duel_response(args: &[u8]) -> Result<DuelResponse, DuelResponseDecodeError> {
    let &[byte] = args else {
        return Err(DuelResponseDecodeError::BadLength(args.len()));
    };
    match byte as i8 {
        0 => Ok(DuelResponse::Decline),
        1 => Ok(DuelResponse::Accept),
        other => Err(DuelResponseDecodeError::UnknownValue(other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_is_accept_zero_is_decline() {
        assert_eq!(decode_send_duel_response(&[1]), Ok(DuelResponse::Accept));
        assert_eq!(decode_send_duel_response(&[0]), Ok(DuelResponse::Decline));
    }

    #[test]
    fn other_values_and_lengths_are_refused() {
        assert_eq!(
            decode_send_duel_response(&[2]),
            Err(DuelResponseDecodeError::UnknownValue(2))
        );
        assert_eq!(
            decode_send_duel_response(&[0xFF]),
            Err(DuelResponseDecodeError::UnknownValue(-1))
        );
        assert_eq!(
            decode_send_duel_response(&[]),
            Err(DuelResponseDecodeError::BadLength(0))
        );
        assert_eq!(
            decode_send_duel_response(&[1, 0]),
            Err(DuelResponseDecodeError::BadLength(2))
        );
    }
}
