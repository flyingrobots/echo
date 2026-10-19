<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Current effect publication control

This is the synthetic zero-effect serialization control retained by the provider
schema tests at Echo commit `09239c7c`. It exercises the current runtime package
checker without trusted host features. It is not public compiler output,
execution evidence, or authority. The package embeds its complete compiler
bundle; tests independently corrupt those bytes and refresh the outer pin to
exercise semantic rejection rather than only identity rejection.
