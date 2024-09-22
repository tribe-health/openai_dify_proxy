# Build stage
FROM rust:1.80 as builder
WORKDIR /usr/src/app
COPY . .
RUN cargo build --release

# Runtime stage
FROM ubuntu:22.04
COPY --from=builder /usr/src/app/target/release/openai_dify_proxy /usr/local/bin/app

RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*

# Create .env file
RUN echo "DIFY_API_URL=${DIFY_API_URL}" > /usr/local/bin/.env

EXPOSE 8223
CMD ["app"]
