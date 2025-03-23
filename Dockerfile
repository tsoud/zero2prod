FROM docker.io/library/rust:1.85 as chef
WORKDIR /app
# Install cargo-chef CLI
RUN cargo install cargo-chef
RUN apt update && apt install lld clang -y

# Planner stage
# Use `chef prepare` to build a recipe
FROM chef as planner
COPY . .
# Compute a lock-like file for the project
RUN cargo chef prepare --recipe-path recipe.json

# Builder stage
FROM chef as builder
COPY --from=planner /app/recipe.json recipe.json
# Build project dependencies
RUN cargo chef cook --release --recipe-path recipe.json
# Use cached layers for build
COPY . .
ENV SQLX_OFFLINE true
RUN cargo build --release --bin zero2prod

# Runtime stage
FROM docker.io/library/debian:bookworm-slim AS runtime
WORKDIR /app
# Need to have OpenSSL and ca-certificates for TLS verification
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates \
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/zero2prod zero2prod
COPY configuration configuration
ENV APP_ENVIRONMENT production
ENTRYPOINT [ "./zero2prod" ]
