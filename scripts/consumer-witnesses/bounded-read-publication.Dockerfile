# SPDX-License-Identifier: Apache-2.0
# © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
FROM rust:1.95.0-bookworm AS compiler
RUN apt-get update && apt-get install -y --no-install-recommends python3 \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /edict
RUN git init && git remote add origin https://github.com/flyingrobots/edict.git \
    && git fetch --depth=1 origin 2405a550e93e1e97fff640caa44bbd0f65ffff3c \
    && git archive FETCH_HEAD | tar -x
RUN cargo build --locked -p edict-cli
WORKDIR /old-echo
RUN git init && git remote add origin https://github.com/flyingrobots/echo.git \
    && git fetch --depth=1 origin 49e9efb68001dfd78563d18bac9359a87671e431 \
    && git archive FETCH_HEAD schemas/edict-provider/package/v1 | tar -x
RUN cp -R schemas/edict-provider/package/v1 /old-provider

FROM rust:1.90.0-bookworm AS provider
WORKDIR /echo
COPY . /echo
RUN cargo run --locked -p echo-wesley-gen --example ordered_publication_witness -- /read-provider

FROM compiler AS witness
COPY --from=provider /read-provider /read-provider
COPY crates/echo-edict-provider-lowerer/tests/fixtures/node-atom-read /read-fixtures
COPY scripts/consumer-witnesses/bounded-read-publication.py /bounded-read-publication.py
CMD ["python3", "/bounded-read-publication.py"]
