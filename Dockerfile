# Build stage
FROM rust:1.70 as builder
WORKDIR /usr/src/app
COPY . .
RUN cargo build --release

# Runtime stage
FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y libssl1.1 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/app/target/release/your_binary_name /usr/local/bin/app

# Create .env file
RUN echo "DIFY_API_URL=${DIFY_API_URL}\nDIFY_API_KEY=${DIFY_API_KEY}" > /usr/local/bin/.env

EXPOSE 8080
CMD ["app"]
