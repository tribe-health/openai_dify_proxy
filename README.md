# OpenAI Dify Proxy

This is a proxy server that forwards requests from OpenAI-compatible clients to a Dify API. It allows you to use OpenAI-compatible tools and libraries with Dify's AI services.

## About the Service

This proxy acts as a bridge between OpenAI's API format and Dify's API. It translates requests from the OpenAI format to Dify's format, and then translates the responses back to the OpenAI format. This allows you to use tools and libraries designed for OpenAI with Dify's AI services.

Key features:

- Supports chat completions
- Handles both streaming and non-streaming responses
- Translates between OpenAI and Dify request/response formats

## Usage

1. Clone the repository
2. Run `cargo run` to start the server
3. Use the proxy server as you would use the OpenAI API, but with the following modifications:
   - Set the base URL to your proxy server's address (e.g., `http://localhost:8223`)
   - Include a `DIFY_API_KEY` header in your requests with your Dify API key

Example using curl:

```bash
bash
curl http://localhost:8080/v1/chat/completions \
-H "Content-Type: application/json" \
-H "DIFY_API_KEY: your_dify_api_key_here" \
-d '{
"model": "dify",
"messages": [{"role": "user", "content": "Hello!"}]
}'
```

## Configuration

The server requires the following environment variables:

- `DIFY_API_URL`: The URL of your Dify API endpoint

You can set these in a `.env` file or in your environment before running the server.

## Note

This proxy is designed to work with Dify's API. Ensure you have the necessary permissions and API access from Dify before using this proxy.
