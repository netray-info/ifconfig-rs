FROM node:22-alpine AS frontend
WORKDIR /build/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend/ .
RUN npm run build

FROM clux/muslrust:stable AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock build.rs ./
COPY src src/
COPY benches benches/
COPY --from=frontend /build/frontend/dist frontend/dist/
RUN cargo build --release --bins && cp $(find /build -xdev -name ifconfig-rs) /

FROM ghcr.io/lukaspustina/ifconfig-rs-data:latest AS data

FROM alpine:3.21
RUN apk add --no-cache ca-certificates wget \
 && addgroup -S ifconfig && adduser -S ifconfig -G ifconfig
WORKDIR /ifconfig-rs
COPY ifconfig.prod.toml ifconfig.toml
COPY --from=builder /ifconfig-rs .
COPY --from=data /data data/
RUN chown -R ifconfig:ifconfig /ifconfig-rs
USER ifconfig
CMD ["./ifconfig-rs", "ifconfig.toml"]
