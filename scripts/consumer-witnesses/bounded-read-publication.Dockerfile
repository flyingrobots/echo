# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
FROM rust:1.95.0-bookworm
RUN apt-get update && apt-get install -y --no-install-recommends python3 \
    && rm -rf /var/lib/apt/lists/*
RUN rustup toolchain install 1.90.0 --profile minimal
WORKDIR /edict
RUN git init && git remote add origin https://github.com/flyingrobots/edict.git \
    && git fetch --depth=1 origin 2405a550e93e1e97fff640caa44bbd0f65ffff3c \
    && git archive FETCH_HEAD | tar -x
WORKDIR /old-echo
RUN git init && git remote add origin https://github.com/flyingrobots/echo.git \
    && git fetch --depth=1 origin 49e9efb68001dfd78563d18bac9359a87671e431 \
    && git archive FETCH_HEAD schemas/edict-provider/package/v1 | tar -x
RUN cp -R schemas/edict-provider/package/v1 /old-provider
WORKDIR /echo
COPY . /echo
COPY crates/echo-edict-provider-lowerer/tests/fixtures/node-atom-read /read-fixtures
COPY scripts/consumer-witnesses/bounded-read-publication.py /bounded-read-publication.py
COPY scripts/consumer-witnesses/bounded-read-runtime.sh /bounded-read-runtime.sh
ENV CARGO_TARGET_DIR=/read-build CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=4 \
    CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
# Builds and evidence belong to the reusable worker, never to image layers.
CMD ["bash", "/bounded-read-runtime.sh"]
