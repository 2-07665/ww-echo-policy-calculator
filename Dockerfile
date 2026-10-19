FROM rust:1.96-bookworm AS build
WORKDIR /source
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY apps ./apps
COPY data ./data
RUN cargo build --locked --release -p echo_server

FROM debian:bookworm-slim
RUN useradd --system --uid 10001 echo
COPY --from=build /source/target/release/echo_server /usr/local/bin/echo-server
USER echo
ENV ECHO_BIND=0.0.0.0:3000
EXPOSE 3000
CMD ["echo-server"]
