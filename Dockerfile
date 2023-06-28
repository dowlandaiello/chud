FROM rust:1.70

COPY src ./src
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

RUN cargo install --path .

ENTRYPOINT ["chudd"]