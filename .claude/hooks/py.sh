# Interpreter launcher for the Python hooks: finds python3, python or py, then execs it.
# Sourced, not run, so it costs no extra process:  set -- <script> <args>; . "<repo>/.claude/hooks/py.sh"
# Windows (Git Bash: OSTYPE msys or cygwin) tries python first, because the Microsoft Store
# python3.exe alias stub exists where python.org installs ship no python3.exe; elsewhere python3 first.
# `hash` is a builtin that finds and remembers the command, so exec needs no second PATH search.
# With no interpreter it warns on stderr and exits 1: a visible, non-blocking hook error (exit 2 would
# block every tool call). Known gap: a Windows host with only the Store stub execs the stub, which fails.
case "${OSTYPE:-}" in
  msys*|cygwin*) _cimm_py_order='python py python3' ;;
  *) _cimm_py_order='python3 python' ;;
esac
for _cimm_py in $_cimm_py_order; do
  if hash "$_cimm_py" 2>/dev/null; then
    exec "$_cimm_py" "$@"
  fi
done
echo "cimmeria hook: no python3, python or py on PATH, so ${1:-a hook} did not run and its guard is off" >&2
exit 1
