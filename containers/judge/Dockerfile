FROM ubuntu:24.04@sha256:4fbb8e6a8395de5a7550b33509421a2bafbc0aab6c06ba2cef9ebffbc7092d90

ARG DEBIAN_FRONTEND=noninteractive
ARG RUST_TOOLCHAIN=1.96.1
ARG CODEX_VERSION=0.144.3

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
       build-essential ca-certificates cmake curl git gnupg jq ninja-build nodejs npm \
       python3 python3-jsonschema python3-yaml xz-utils \
    && curl -fsSL https://apt.llvm.org/llvm-snapshot.gpg.key \
       -o /usr/share/keyrings/apt.llvm.org.asc \
    && echo "deb [signed-by=/usr/share/keyrings/apt.llvm.org.asc] https://apt.llvm.org/noble/ llvm-toolchain-noble-21 main" \
       > /etc/apt/sources.list.d/llvm-21.list \
    && apt-get update \
    && apt-get install -y --no-install-recommends \
       clang-21 libclang-21-dev llvm-21-dev \
    && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs \
      | sh -s -- -y --profile minimal --default-toolchain "${RUST_TOOLCHAIN}" \
    && npm install --global "@openai/codex@${CODEX_VERSION}"

ENV PATH="/root/.cargo/bin:/opt/whyvec-install/bin:${PATH}"
WORKDIR /opt/whyvec
COPY . .

RUN cargo build --workspace --release \
    && cargo install --path crates/whyvec-cli --root /opt/whyvec-install \
    && python3 scripts/build_helpers.py --output /opt/whyvec-install/bin \
    && codex plugin marketplace add /opt/whyvec --json \
    && codex plugin add whyvec@whyvec-local --json \
    && whyvec doctor --format json | jq -e '.ready == true' \
    && python3 scripts/validate_repository.py \
    && python3 scripts/check_portable_evidence.py

ENTRYPOINT ["/opt/whyvec/scripts/demo"]
CMD ["--ci"]
