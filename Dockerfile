FROM rust:1.70

COPY src ./src
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

RUN cargo install --path .

ENV RUST_LOG="debug"

EXPOSE 8080
EXPOSE 6224

HEALTHCHECK --interval=5m --timeout=3s \
  CMD curl -f http://localhost:8080/health_check || exit 1

ENTRYPOINT ["chudd"]