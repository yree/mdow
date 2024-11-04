FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin mdow

FROM flyio/litefs:0.5 AS runtime
# Install required packages
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/mdow /usr/local/bin
COPY litefs.yml /etc/litefs.yml

ENV DATABASE_URL="sqlite:/litefs/mdow.db"
ENTRYPOINT ["/usr/local/bin/litefs"]
