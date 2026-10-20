# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
# Run a command in its own process group; kill it on deadline or if ROOT exceeds a byte cap.
import os, signal, subprocess, sys, time
root, cap_gib, deadline_s = sys.argv[1], float(sys.argv[2]), int(sys.argv[3]); cmd = sys.argv[4:]
def used(p):
    t = 0
    for d, _, fs in os.walk(p):
        for f in fs:
            try: t += os.lstat(os.path.join(d, f)).st_blocks * 512
            except OSError: pass
    return t
p = subprocess.Popen(cmd, start_new_session=True); start = time.time(); reason = None
while p.poll() is None:
    time.sleep(10)
    u = used(root)
    if u > cap_gib * 2**30: reason = f"disk cap: {u/2**30:.1f} GiB"
    elif time.time() - start > deadline_s: reason = "deadline"
    if reason:
        print("GUARD STOP:", reason, flush=True); os.killpg(p.pid, signal.SIGTERM); time.sleep(5)
        try: os.killpg(p.pid, signal.SIGKILL)
        except ProcessLookupError: pass
        break
p.wait(); print(f"GUARD END rc={p.returncode} used={used(root)/2**30:.2f} GiB elapsed={time.time()-start:.0f}s", flush=True)
sys.exit(1 if reason else p.returncode)
