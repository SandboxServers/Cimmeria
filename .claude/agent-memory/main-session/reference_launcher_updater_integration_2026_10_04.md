# Desktop updater integration

The signed updater is a separate durable owner within DesktopState. Guard early
admission before preparatory writes; `operations_mut()` alone is too late and
Install can bypass it through an injected commit. Integrated guards cover Install,
Play, Repair, uninstall, runtime preparation, import, failed-install cleanup and
repair-backup cleanup. Directory changes are gated; summary consent is not.

The updater holds a snapshot of the operation revision. Refresh it when the
existing Play observer reports a changed revision, or its next Check/Download
will be stale after another operation. Avoid unconditional reciprocal view
refreshes. The production-composition/native-updater UAT covers this interaction.

Source: crates/launcher/desktop/docs/updater.md. A verified saved package is not
an installed update; production configuration and Apply/recovery are still open.
