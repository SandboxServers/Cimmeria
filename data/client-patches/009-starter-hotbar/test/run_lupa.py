"""Run the 009-starter-hotbar logic UAT (run.lua) under Lua 5.1 without a system lua5.1.

Windows has no packaged Lua 5.1 interpreter; `lupa` ships one:

    python -m pip install lupa
    python data/client-patches/009-starter-hotbar/test/run_lupa.py

Set SGW_UI_DIR (and SGW_V26_ACTIONPROFILES) to run the real-script variants;
see run.lua. CI runs run.lua directly with Ubuntu's lua5.1.
"""

import os
import sys

import lupa.lua51 as lua51

here = os.path.dirname(os.path.abspath(__file__))
script = os.path.join(here, "run.lua").replace("\\", "/")

runtime = lua51.LuaRuntime()
runtime.execute("arg = { [0] = %r }" % script)
# os.exit inside lupa would end the Python process without a status we can
# read, so capture the exit code instead.
runtime.execute("__exit_code = 0; os.exit = function(code) __exit_code = code or 0; error('__exit') end")
try:
    runtime.execute("dofile(arg[0])")
except lua51.LuaError as err:
    if "__exit" not in str(err):
        raise
sys.exit(int(runtime.eval("__exit_code")))
