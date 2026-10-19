# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
# For stacks under ROOT, attribute blake3 samples (outermost blake3 frame) to the nearest warp_core ancestor.
import re, sys, collections
path, root = sys.argv[1], sys.argv[2]
lines = open(path).read().split("Call graph:")[1].split("Total number in stack")[0].splitlines()
pat = re.compile(r'^([\s+!:|]*)(\d+)\s+(.+?)\s+\(in ')
by = collections.Counter(); stack = []; tot = 0; blk = 0
for l in lines:
    m = pat.match(l)
    if not m: continue
    d, c, n = len(m.group(1)), int(m.group(2)), re.sub(r'::h[0-9a-f]{16}$', '', m.group(3))
    while stack and stack[-1][0] >= d: stack.pop()
    names = [s[1] for s in stack]
    under = any(root in x for x in names)
    if root in n and not under: tot += c
    if under and n.startswith('blake3') and not any(x.startswith('blake3') for x in names):
        caller = next((x for x in reversed(names) if x.startswith('warp_core') or x.startswith('echo')), names[-1])
        by[caller] += c; blk += c
    stack.append((d, n))
print(f"root {root}: {tot} samples, blake3 {blk} ({100*blk/max(tot,1):.1f}%)")
for n, c in by.most_common(15): print(f"{c:6d} {100*c/tot:5.1f}%  {n[:120]}")
