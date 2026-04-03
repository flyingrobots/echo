<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Explicit negative test mapping for decoder controls

Ref: #279

Map every decoder control (CBOR boundary validation, envelope
rejection, malformed input handling) to an explicit negative test.
Currently the security claims (`sec-claim-map.json`) reference CI
gates but don't guarantee every decoder edge case has a dedicated
negative test.
