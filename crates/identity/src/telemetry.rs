//! The C11 spans of the sign-in flow, one helper per span.
//!
//! Each helper returns a `tracing::Span` with the C11 fields declared `Empty`,
//! so the caller records them as the request goes. The fields are `provider`,
//! `outcome`, `reason`, `account_id` and `latency_ms`: never a code, token,
//! ticket, subject, email, display name or IP (contract C11, R7).

use std::time::Instant;

use tracing::Span;

/// Records the outcome, reason and latency on `span`. `started` is when the
/// request began; the latency is whole milliseconds.
pub fn finish(span: &Span, outcome: &'static str, reason: &'static str, started: Instant) {
    span.record("outcome", outcome);
    span.record("reason", reason);
    span.record("latency_ms", started.elapsed().as_millis() as u64);
}

/// `auth.idp.authorize`: the provider's authorize redirect.
pub fn idp_authorize() -> Span {
    tracing::info_span!(
        target: "cimmeria_identity",
        "auth.idp.authorize",
        provider = tracing::field::Empty,
        outcome = tracing::field::Empty,
        reason = tracing::field::Empty,
        latency_ms = tracing::field::Empty,
    )
}

/// `auth.idp.callback`: the provider's redirect back to the server.
pub fn idp_callback() -> Span {
    tracing::info_span!(
        target: "cimmeria_identity",
        "auth.idp.callback",
        provider = tracing::field::Empty,
        outcome = tracing::field::Empty,
        reason = tracing::field::Empty,
        latency_ms = tracing::field::Empty,
    )
}

/// `auth.idp.token`: the `/token` exchange of a handoff code or refresh token.
pub fn idp_token() -> Span {
    tracing::info_span!(
        target: "cimmeria_identity",
        "auth.idp.token",
        outcome = tracing::field::Empty,
        reason = tracing::field::Empty,
        account_id = tracing::field::Empty,
        latency_ms = tracing::field::Empty,
    )
}

/// `auth.idp.link`: a link of a provider identity to an account, with whether
/// the link cleared a password (C15).
pub fn idp_link() -> Span {
    tracing::info_span!(
        target: "cimmeria_identity",
        "auth.idp.link",
        provider = tracing::field::Empty,
        outcome = tracing::field::Empty,
        reason = tracing::field::Empty,
        account_id = tracing::field::Empty,
        password_cleared = tracing::field::Empty,
        latency_ms = tracing::field::Empty,
    )
}

/// `auth.idp.unlink`: the removal of a linked provider identity.
pub fn idp_unlink(account_id: u32) -> Span {
    tracing::info_span!(
        target: "cimmeria_identity",
        "auth.idp.unlink",
        account_id,
        provider = tracing::field::Empty,
        outcome = tracing::field::Empty,
        reason = tracing::field::Empty,
        latency_ms = tracing::field::Empty,
    )
}

/// `auth.launch_ticket.mint`: a launch ticket minted for the launcher.
pub fn launch_ticket_mint(account_id: u32) -> Span {
    tracing::info_span!(
        target: "cimmeria_identity",
        "auth.launch_ticket.mint",
        account_id,
        outcome = tracing::field::Empty,
        reason = tracing::field::Empty,
        latency_ms = tracing::field::Empty,
    )
}

/// `auth.launch_ticket.consume`: a launch ticket spent by the login handshake.
pub fn launch_ticket_consume() -> Span {
    tracing::info_span!(
        target: "cimmeria_identity",
        "auth.launch_ticket.consume",
        account_id = tracing::field::Empty,
        outcome = tracing::field::Empty,
        reason = tracing::field::Empty,
        latency_ms = tracing::field::Empty,
    )
}

/// `auth.launch_ticket.end`: the launcher's end-of-session call.
pub fn launch_ticket_end(account_id: u32) -> Span {
    tracing::info_span!(
        target: "cimmeria_identity",
        "auth.launch_ticket.end",
        account_id,
        outcome = tracing::field::Empty,
        reason = tracing::field::Empty,
        latency_ms = tracing::field::Empty,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing::span::{Attributes, Id, Record};
    use tracing::{Event, Metadata, Subscriber};

    /// Enables every span, so the helpers hand out real spans with metadata.
    struct Enabled;

    impl Subscriber for Enabled {
        fn enabled(&self, _: &Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _: &Attributes<'_>) -> Id {
            Id::from_u64(1)
        }
        fn record(&self, _: &Id, _: &Record<'_>) {}
        fn record_follows_from(&self, _: &Id, _: &Id) {}
        fn event(&self, _: &Event<'_>) {}
        fn enter(&self, _: &Id) {}
        fn exit(&self, _: &Id) {}
    }

    /// The field names of every C11 span helper.
    fn all_fields() -> Vec<(&'static str, Vec<&'static str>)> {
        tracing::subscriber::with_default(Enabled, || {
            let spans = [
                ("authorize", idp_authorize()),
                ("callback", idp_callback()),
                ("token", idp_token()),
                ("link", idp_link()),
                ("unlink", idp_unlink(1)),
                ("mint", launch_ticket_mint(1)),
                ("consume", launch_ticket_consume()),
                ("end", launch_ticket_end(1)),
            ];
            spans
                .iter()
                .map(|(name, span)| {
                    let meta = span.metadata().expect("an enabled span has metadata");
                    (*name, meta.fields().iter().map(|f| f.name()).collect())
                })
                .collect()
        })
    }

    /// No C11 span carries a field that could hold a secret or an identity.
    #[test]
    fn span_fields_never_carry_secrets_or_identity() {
        const FORBIDDEN: [&str; 7] = [
            "code",
            "token",
            "ticket",
            "subject",
            "email",
            "ip",
            "display_name",
        ];
        for (span, fields) in all_fields() {
            for field in fields {
                assert!(
                    !FORBIDDEN.contains(&field),
                    "span {span} declares forbidden field {field}"
                );
            }
        }
    }

    /// The link span keeps the C15 `password_cleared` flag that its route records.
    #[test]
    fn link_span_declares_password_cleared() {
        let fields = all_fields()
            .into_iter()
            .find(|(span, _)| *span == "link")
            .map(|(_, fields)| fields)
            .expect("the link span is listed");
        assert!(fields.contains(&"password_cleared"));
    }

    /// Every helper declares the C11 outcome and latency fields it records.
    #[test]
    fn every_span_declares_outcome_and_latency() {
        for (span, fields) in all_fields() {
            assert!(fields.contains(&"outcome"), "span {span} lacks outcome");
            assert!(
                fields.contains(&"latency_ms"),
                "span {span} lacks latency_ms"
            );
        }
    }
}
