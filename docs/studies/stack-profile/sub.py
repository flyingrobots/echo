# SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
# Inclusive samples per function, counting only stacks under ROOT (outermost occurrence).
import re, sys, collections
path, root, top = sys.argv[1], sys.argv[2], int(sys.argv[3])
lines = open(path).read().split("Call graph:")[1].split("Total number in stack")[0].splitlines()
pat = re.compile(r'^([\s+!:|]*)(\d+)\s+(.+?)\s+\(in ')
incl = collections.Counter(); stack = []; tot = 0
for l in lines:
    m = pat.match(l)
    if not m: continue
    d, c, n = len(m.group(1)), int(m.group(2)), re.sub(r'::h[0-9a-f]{16}$', '', m.group(3))
    while stack and stack[-1][0] >= d: stack.pop()
    names = [s[1] for s in stack]
    if any(root in x for x in names):
        if n not in names: incl[n] += c
    elif root in n:
        tot += c
    stack.append((d, n))
print("root", root, "samples", tot)
for n, c in incl.most_common(top): print(f"{c:6d} {100*c/tot:5.1f}%  {n[:130]}")
