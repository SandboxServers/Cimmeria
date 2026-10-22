# LN-20b worknote

- Step 1-2 (dialog_overrides): added overrides_bank.rs with VAULT_EXPAND_DIALOG; mod.rs appends it to DIALOG_OVERRIDES, deletes QUARANTINED_DIALOG_OVERRIDES, updates the expand/debug-hub tests. Commit part 1. Lane checks skipped per owner override.
- Steps 3-8 (seed agreement tests, resources tests, wire vault.rs, cell-interactions expand.rs and expand_tests.rs): quarantine symbols removed, served-offer test replaces the suppressed one. Commit part 2. Next: step 9 (cell bank.rs test), 10 (bank_dispatch_tests), 11 (grep).
