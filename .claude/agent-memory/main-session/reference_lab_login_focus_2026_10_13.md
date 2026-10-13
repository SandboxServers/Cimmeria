# Login input focus recovery (2026-10-13)

`lab_login` checks the real desktop foreground HWND before each credential field. It requests foreground once if the client is not active and verifies again. It then locates the edit rectangle and uses `Root:getTargetChildAtPosition` through the shared CEGUI hit-test before calling the typing path. A covering window is reported by name and prevents credential input. Both account and password text are redacted from flow steps and retry logs.

The regression tests use a fake bridge to prove one refocus and to refuse a covered login edit before any input method is called. The recovery probe against a live client remains pending for the coordinator.