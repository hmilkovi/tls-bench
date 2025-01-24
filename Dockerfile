FROM rust:alpine AS build
COPY . /app
WORKDIR /app
RUN apk add --no-cache musl-dev
RUN cargo build --release

FROM alpine as runtime
COPY --from=build /app/target/release/tls-bench /bin/tls-bench
ENTRYPOINT ["tls-bench"]
