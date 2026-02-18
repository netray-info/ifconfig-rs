FROM node:22-alpine AS frontend
WORKDIR /build/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend/ .
RUN npm run build

FROM clux/muslrust:latest AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock build.rs ./
COPY src src/
COPY --from=frontend /build/frontend/dist frontend/dist/
RUN cargo build --release && cp $(find /build -xdev -name ifconfig-rs) /

FROM ghcr.io/lukaspustina/ifconfig-rs-data:latest AS data

FROM alpine:latest
WORKDIR /ifconfig-rs
COPY ifconfig.prod.toml ifconfig.toml
COPY --from=builder /ifconfig-rs .
COPY --from=data /data data/
CMD ["./ifconfig-rs", "ifconfig.toml"]
