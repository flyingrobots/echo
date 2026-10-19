# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
# Recompute Echo application result identities (echo_operation.rs application_result_identity):
# blake3("echo:operation-application-result:v1\0" || projection || u64le(len)||type || u64le(len)||bytes)
import json, subprocess, sys
fixtures, projection, out_type = sys.argv[1], bytes.fromhex(sys.argv[2]), sys.argv[3].encode()
def ident(canonical_hex):
    data = b"echo:operation-application-result:v1\0" + projection
    for part in (out_type, bytes.fromhex(canonical_hex)):
        data += len(part).to_bytes(8, "little") + part
    return subprocess.run(["/Users/j/.cargo/bin/b3sum", "--no-names"], input=data, capture_output=True, check=True).stdout.decode().split()[0]
cases = json.load(open(fixtures))
print(json.dumps({name: {"canonicalBytesHex": c["canonicalBytesHex"], "resultIdentity": ident(c["canonicalBytesHex"])} for name, c in cases.items()}, indent=2))
