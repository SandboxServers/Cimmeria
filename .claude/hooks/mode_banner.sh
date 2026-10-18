# SessionStart mode banner: tells the user and the model which persona this session runs as.
# Pure POSIX sh with builtins only (no Python, no forks), so it still works where the Python hooks cannot.
# First SessionStart entry in .claude/settings.json, with no matcher, so it also runs after a compaction.
payload=''
while IFS= read -r line || [ -n "$line" ]; do
  payload="$payload$line"
done
agent=''
case "$payload" in
  *'"agent_type":"'*) rest=${payload#*'"agent_type":"'}; agent=${rest%%'"'*} ;;
  *'"agent_type": "'*) rest=${payload#*'"agent_type": "'}; agent=${rest%%'"'*} ;;
esac
case "$agent" in
  *[!A-Za-z0-9_-]*) agent='' ;;
esac
guards=''
if ! hash python3 2>/dev/null && ! hash python 2>/dev/null && ! hash py 2>/dev/null; then
  guards=' WARNING: no python3, python or py on PATH, so the shell and orchestrator guards are off.'
fi
if [ -z "$agent" ] || [ "$agent" = orchestrator ]; then
  msg="Mode: orchestrator, this repo's default persona (it dispatches agents and has no shell). For direct work start claude --agent hands-on. How the AI workflow works: README.md#how-ai-development-works-here.$guards"
  ctx="Session mode: you run as the orchestrator persona unless this session was started with --agent. When the user asks for direct work, mention once that claude --agent hands-on starts a full-tool session, then still dispatch the right agent for the task.$guards"
  printf '{"systemMessage":"%s","hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":"%s"}}\n' "$msg" "$ctx"
else
  printf '{"systemMessage":"%s"}\n' "Mode: $agent (started with --agent).$guards"
fi
exit 0
