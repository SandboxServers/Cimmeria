---
name: reference-client-ui-lua-overlay-testing
description: "Client UI Lua (SGWGame/Content/UI) facts learned in BM-05, 2026-09-27: files are ASCII+CRLF, Debug:log is the client's Lua logger, named windows are Lua globals, the stock BlackMarket layout lacks BlackMarket_ErrorText; how to run a Lua 5.1 logic UAT on Windows (lupa)."
metadata:
  type: reference
---

Learned while patching the Black Market UI for BM-05 (2026-09-27), from the QA client's `Content/UI` tree:

- **Encoding.** The stock `.lua` and `.layout` files are plain ASCII with CRLF, even though the Lua host (`lua51.dll`) is wide-string. An overlay copy keeps both: `.gitattributes` marks `crates/client-patches/overlay/Content/**` `-text`, because this machine runs `core.autocrlf=true`.
- **Logging.** The only Lua logger the stock UI uses is `Debug:log(text)` (`Common/Background/Background.lua`). Whether it reaches `sgwdebuglog*` or the sessions log that the launcher tails is **not verified**.
- **Window globals.** Every named layout window is a Lua global. `LayoutImport Prefix="X"` makes child `Name="Y"` into global `XY` (for example `BlackMarket_Search1MainContainer`). A wrong prefix is a nil global. That is how U7 hid.
- **Stock bug outside the U-list.** `BlackMarket.lua` writes `BlackMarket_ErrorText` in several places, but `BlackMarket.layout` never defines it. The overlay adds it.
- **Lua 5.1 on Windows.** There is no packaged interpreter, and WSL Ubuntu's dpkg was broken at the time. `pip install lupa` and `import lupa.lua51` gives a real 5.1 (`crates/client-patches/overlay/test/run_lupa.py`). CI uses Ubuntu's `lua5.1` package.
- **Harness pattern.** Build the stub windows from the patched `.layout` itself (named windows plus imported row children), so a name the Lua gets wrong fails the test the way it fails in the client. Load the overlay with `setfenv(loadfile(...), env)`, with `env._G = env`.

Related: [[reference-client-idle-send-cadence]]
