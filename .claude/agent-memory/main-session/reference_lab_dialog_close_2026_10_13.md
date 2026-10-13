# Dialog Done closure

Issue #1706 reproduced a one-page Cellblock pistol dialog whose visible Done button accepted a click without hiding `DialogWin`; the native `DialogWin/DialogWin__auto_closebutton__` closed it. `lab_finish_dialog` must read visibility after Done and after a native close fallback. A click acknowledgement alone does not establish that the dialog finished.
