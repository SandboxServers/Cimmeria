# Dialog Done closure

Issue #1706 reproduced a one-page Cellblock pistol dialog whose visible Done button accepted a click without hiding `DialogWin`; the native `DialogWin/DialogWin__auto_closebutton__` closed it. `lab_finish_dialog` must read visibility after Done and after a native close fallback. A click acknowledgement alone does not establish that the dialog finished.

Local lab UAT (2026-10-13, fresh Praxis Commando, God mode on): the two-page intro and one-page Frost dialogs closed with `dialog_still_open=false`. Searching the Guard opened tutorial 5882 over one-page dialog 3996; `lab_finish_dialog` failed at `done` with `covered-target` instead of reporting success. Closing `TutorialWin`, then rerunning `lab_finish_dialog`, closed `DialogWin` and returned `dialog_still_open=false`. The native-close fallback was covered by the flow regression test, not triggered by this run.
