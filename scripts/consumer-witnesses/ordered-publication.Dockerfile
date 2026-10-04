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
WORKDIR /consumer-source
RUN git init && git remote add origin https://github.com/flyingrobots/jedit.git \
    && git fetch --depth=1 origin 19edb6fba94a8fea2dea63aa2f05cffc3e084f97 \
    && git archive FETCH_HEAD edict/replace-range-probes/state-read | tar -x
WORKDIR /echo-source
RUN git init && git remote add origin https://github.com/flyingrobots/echo.git \
    && git fetch --depth=1 origin 49e9efb68001dfd78563d18bac9359a87671e431 \
    && git archive FETCH_HEAD schemas/edict-provider/package/v1 | tar -x

FROM rust:1.90.0-bookworm AS provider
WORKDIR /echo
COPY . /echo
RUN cargo run --locked -p echo-wesley-gen --example ordered_publication_witness -- /ordered-provider

FROM compiler AS witness
COPY --from=provider /ordered-provider /ordered-provider
COPY scripts/consumer-witnesses/ordered-publication.py /ordered-publication.py
CMD ["python3", "/ordered-publication.py"]
