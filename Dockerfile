FROM docker.io/library/rust:1.85

WORKDIR /app

RUN apt update && apt install lld clang -y

COPY . .

ENV SQLX_OFFLINE true

RUN cargo build --release

ENTRYPOINT [ "./target/release/zero2prod" ]
