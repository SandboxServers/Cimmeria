# Launcher release identity

Owner-approved D-LX15 supersedes D-LX8: stable UTC YYYY.M.(D*100+N), slots
1..99, all releases require Minisign signatures. Old stable and historical
prerelease installs accept these versions without weakening the stable-only
updater guard. Windows file-version numeric components are 16-bit; maximum
patch is 3199. MSI uses different limits and is not shipped.

The stamp workflow reserves identity before building; reruns reuse its run
artifact. Never cut a release without separate owner authorization.

Applied overlay identity must come from retained verified installed release
and live ledger, not catalog offers. SgwGame patches use id@sgw_game keys;
incomplete overlays must be reported separately from fully applied ones.
