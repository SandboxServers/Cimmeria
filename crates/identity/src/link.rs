//! Account linking (contract C7): which account a provider sign-in lands on.
//!
//! Three rules, applied in order by [`resolve`]:
//!
//! 1. A provider identity already bound to an account signs in to that
//!    account, unless the account is disabled, which refuses the sign-in.
//! 2. Only when the provider is admin-trusted, it verified the email, and
//!    exactly one linkable account holds a trusted, verified identity with
//!    that email, does the sign-in link automatically to that account.
//! 3. Anything else needs the player to link by hand. The page only hints
//!    at an existing account (`EmailMatch`) when the same trusted, verified
//!    email is on file.
//!
//! Residual risk: auto-link trusts the provider's verified email. An attacker
//! who controls a trusted provider account that carries the victim's verified
//! email gets the victim's account. That is why only admin-trusted providers
//! auto-link, and why staff and test accounts never do.

/// Accounts at or above this access level are staff and never auto-link.
pub const STAFF_ACCESSLEVEL_MIN: i32 = 1;

/// The facts about one local account that linking looks at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountFacts {
    /// The account's id.
    pub account_id: u32,
    /// Whether the account may sign in.
    pub enabled: bool,
    /// The account's access level; `>= STAFF_ACCESSLEVEL_MIN` is staff.
    pub accesslevel: i32,
    /// Whether the account is a test account.
    pub test_account: bool,
}

/// What the sign-in and the database say about one provider identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkFacts {
    /// The account already holding this (provider, issuer, subject).
    pub bound_to: Option<AccountFacts>,
    /// Distinct accounts with a trusted, verified identity of the same
    /// lower-cased email.
    pub email_candidates: Vec<AccountFacts>,
    /// Whether the provider is admin-trusted for its email claim.
    pub provider_trusted_email: bool,
    /// Whether the provider reported the email as verified.
    pub email_verified: bool,
}

/// The outcome of resolving one provider sign-in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    /// Sign in to the bound account.
    Existing(u32),
    /// Link the identity to this account, then sign in.
    AutoLink(u32),
    /// The player must link by hand; `hint` says whether to show a match.
    NeedsLink {
        /// Whether the login page may point at an existing account.
        hint: LinkHint,
    },
    /// The sign-in is refused.
    Refused(Refusal),
}

/// Whether the link page may mention an existing account.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkHint {
    /// No hint is shown.
    None,
    /// A trusted, verified email matches an existing account.
    EmailMatch,
}

/// Why a sign-in is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The bound account is disabled.
    AccountDisabled,
}

/// Resolves one provider sign-in by the three rules in the module doc.
pub fn resolve(facts: &LinkFacts) -> Resolution {
    if let Some(bound) = &facts.bound_to {
        return if bound.enabled {
            Resolution::Existing(bound.account_id)
        } else {
            Resolution::Refused(Refusal::AccountDisabled)
        };
    }
    let trusted_match = facts.provider_trusted_email && facts.email_verified;
    if trusted_match && facts.email_candidates.len() == 1 {
        let only = &facts.email_candidates[0];
        if linkable(only) {
            return Resolution::AutoLink(only.account_id);
        }
    }
    let hint = if trusted_match && !facts.email_candidates.is_empty() {
        LinkHint::EmailMatch
    } else {
        LinkHint::None
    };
    Resolution::NeedsLink { hint }
}

/// Whether an account may be auto-linked: enabled, not staff, not a test
/// account.
pub fn linkable(a: &AccountFacts) -> bool {
    a.enabled && a.accesslevel < STAFF_ACCESSLEVEL_MIN && !a.test_account
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account(id: u32) -> AccountFacts {
        AccountFacts {
            account_id: id,
            enabled: true,
            accesslevel: 0,
            test_account: false,
        }
    }

    fn facts(bound: Option<AccountFacts>, candidates: Vec<AccountFacts>) -> LinkFacts {
        LinkFacts {
            bound_to: bound,
            email_candidates: candidates,
            provider_trusted_email: true,
            email_verified: true,
        }
    }

    #[test]
    fn bound_identity_signs_in_or_is_refused() {
        assert_eq!(
            resolve(&facts(Some(account(7)), vec![])),
            Resolution::Existing(7)
        );
        let disabled = AccountFacts {
            enabled: false,
            ..account(7)
        };
        assert_eq!(
            resolve(&facts(Some(disabled), vec![])),
            Resolution::Refused(Refusal::AccountDisabled)
        );
    }

    #[test]
    fn auto_links_one_trusted_verified_linkable_match() {
        assert_eq!(
            resolve(&facts(None, vec![account(9)])),
            Resolution::AutoLink(9)
        );
    }

    #[test]
    fn never_auto_links_staff_test_disabled_or_ambiguous() {
        let no_link = |f: LinkFacts| {
            assert!(
                matches!(resolve(&f), Resolution::NeedsLink { .. }),
                "expected NeedsLink for {f:?}"
            );
        };
        let staff = AccountFacts {
            accesslevel: 1,
            ..account(1)
        };
        no_link(facts(None, vec![staff]));
        let test = AccountFacts {
            test_account: true,
            ..account(2)
        };
        no_link(facts(None, vec![test]));
        let disabled = AccountFacts {
            enabled: false,
            ..account(3)
        };
        no_link(facts(None, vec![disabled]));
        no_link(facts(None, vec![account(4), account(5)]));
        let mut untrusted = facts(None, vec![account(6)]);
        untrusted.provider_trusted_email = false;
        no_link(untrusted);
        let mut unverified = facts(None, vec![account(6)]);
        unverified.email_verified = false;
        no_link(unverified);
    }

    #[test]
    fn hint_only_for_trusted_verified_matches() {
        let hint_of = |f: &LinkFacts| match resolve(f) {
            Resolution::NeedsLink { hint } => hint,
            other => panic!("expected NeedsLink, got {other:?}"),
        };
        // Two candidates: no auto-link, but the trusted verified match hints.
        let two = facts(None, vec![account(4), account(5)]);
        assert_eq!(hint_of(&two), LinkHint::EmailMatch);
        let mut untrusted = facts(None, vec![account(4)]);
        untrusted.provider_trusted_email = false;
        assert_eq!(hint_of(&untrusted), LinkHint::None);
        let mut unverified = facts(None, vec![account(4)]);
        unverified.email_verified = false;
        assert_eq!(hint_of(&unverified), LinkHint::None);
        assert_eq!(hint_of(&facts(None, vec![])), LinkHint::None);
    }
}
