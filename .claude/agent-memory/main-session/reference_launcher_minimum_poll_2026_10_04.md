# Launcher minimum survives polling

The desktop Play view polls native status every second. Mapping only the
`launcher_too_old` mutation error was insufficient: successful inspection erased
it and re-enabled Play. `LaunchStatus.launcher_update_required` now carries the
authenticated installed-release gate during idle inspection and withholds the
installation capability. Native signed fixtures plus Effect/native persistence
UAT cover repeated polls and reopening with compiled identity changes.

Do not read installed owner evidence while a live operation holds its Windows
exclusive handle: the minimum-status read is restricted to idle state. Actual
updater replacement and packaged visuals are separate evidence.

Native Windows run `37218617196` exposed a separate test-fixture issue: recursive
before/after snapshots tried to read the held `launcher.lock` through another
handle (OS error 33). Minimum and updater-exclusion snapshots now retain the lock
entry and assert its empty length via metadata, while reading every state payload
byte normally. Four minimum tests and the updater-exclusion test pass locally;
the correction still requires its native Windows rerun.
