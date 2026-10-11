#!/usr/bin/env bash
# Runs before every job. Logs the disk picture so a full volume shows up in the
# job log, not as a mystery "No space left on device" halfway through a build.
set -u
echo "runner $(hostname): workspace $(df -h --output=avail /home/runner/_work | tail -1 | tr -d ' ') free," \
     "sccache $(du -sh /home/runner/sccache 2>/dev/null | cut -f1)"
exit 0
