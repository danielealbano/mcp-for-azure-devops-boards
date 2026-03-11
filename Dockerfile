FROM rust:1.91.1-alpine AS builder
WORKDIR /app

# Build

RUN apk update && apk add --no-cache pkgconf openssl-dev openssl-libs-static

# Very ugly trick to fetch the dependencis without the need of actually copying the source code
COPY Cargo.toml Cargo.lock ./
COPY mcp-tools-codegen/Cargo.toml mcp-tools-codegen/Cargo.toml
RUN mkdir -p src mcp-tools-codegen/src \
    && echo 'fn main() {}' > src/main.rs \
    && echo 'pub fn dummy() {}' > mcp-tools-codegen/src/lib.rs

RUN cargo fetch

COPY . .

RUN cargo build --release --bin mcp-for-azure-devops-boards

# Runtime

FROM alpine:3.22 AS runtime

LABEL com.github.actions.name="auto publish"
LABEL com.github.actions.icon="package"
LABEL com.github.actions.color="blue"

LABEL version="0.5.0"
LABEL repository="https://github.com/danielealbano/mcp-for-azure-devops-boards"
LABEL homepage="https://github.com/danielealbano/mcp-for-azure-devops-boards"
LABEL maintainer="Daniele Salvatore Albano <d.albano@gmail.com>"

RUN apk update && apk add --no-cache ca-certificates tzdata

WORKDIR /app

COPY --from=builder /app/target/release/mcp-for-azure-devops-boards /usr/local/bin/mcp-for-azure-devops-boards

ENTRYPOINT ["mcp-for-azure-devops-boards"]
