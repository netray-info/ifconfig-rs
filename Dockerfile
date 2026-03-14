FROM node:22-alpine AS frontend
WORKDIR /build/frontend
COPY frontend/package.json frontend/package-lock.json frontend/.npmrc ./
RUN --mount=type=secret,id=NODE_AUTH_TOKEN,env=NODE_AUTH_TOKEN npm ci
COPY frontend/ .
RUN npm run build

FROM clux/muslrust:stable AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock build.rs ./
COPY src src/
COPY benches benches/
COPY --from=frontend /build/frontend/dist frontend/dist/
RUN cargo build --release --bins && cp $(find /build -xdev -name ifconfig-rs) /

FROM ghcr.io/netray-info/ifconfig-rs-data:latest AS data

FROM alpine:3.21
RUN apk add --no-cache ca-certificates wget \
 && addgroup -S ifconfig && adduser -S ifconfig -G ifconfig
WORKDIR /ifconfig-rs
COPY ifconfig.example.toml ifconfig.toml
ENV IFCONFIG_SERVER__BIND=0.0.0.0:8000
COPY --from=builder /ifconfig-rs .
COPY --from=data /data data/
RUN chown -R ifconfig:ifconfig /ifconfig-rs
USER ifconfig
CMD ["./ifconfig-rs", "ifconfig.toml"]
