---
name: lab-uat-runner-in-process-tool-calls
description: How the cimmeria-lab UAT runner calls its own MCP tools by name in-process (rmcp 3.4 RequestContext), plus chat-mark and spec traps found building it
metadata:
  type: reference
---

- rmcp 3.4: a `#[tool]` fn can take `ctx: RequestContext<RoleServer>` as an extractor; with it,
  `self.tool_router.call(ToolCallContext::new(self, CallToolRequestParams::new(name).with_arguments(obj), ctx.clone()))`
  dispatches any routed tool by name (see `crates/lab/src/server/uat.rs` RouterInvoker). `Peer::new` is
  `pub(crate)`, so outside a handler the only way to get a peer is `serve_directly` over a duplex.
- Result normalisation: serialize `CallToolResult` to JSON and walk `content[]` (text → JSON parse, image → base64).
- Chat `since` marks must be read *before* the labelled action, not after: a fast reply lands before an after-mark.
- Tool names planned by sibling changes live in one table (`crates/lab/src/uat/tools.rs`, `@alias` in specs);
  a rename is one line there. Unknown driving tools must declare their tier in the spec.
- `r#"..."#` test fixtures break on regexes containing `"#`; use `r##"..."##`.
- Spec regexes with `${var}` must be validated with the vars stripped (`spec::without_vars`).

Related: [[lab-probe-traffic-starves-watchdog]]
