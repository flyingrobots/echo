<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Public compiler negation fixture

The source is the actual Jedit flat-text replacement reference with a bounded
range guard followed by a negated comparison rejecting unchanged text.
The checked-in hex files are unchanged bytes emitted by the public Edict
application build and independently accepted by the selected Echo provider.
`provenance.json` records producer coordinates, raw SHA-256 hashes and the
operation package's domain-separated pin.

The runtime test checks literal replacement outputs and rejects no-op inputs
and invalid ranges. It does not establish UTF-8 admission, persistent rope
editing, atomic effects or recovery.
