# OpenAI Dify Proxy

A high-performance proxy server that enables OpenAI-compatible clients to seamlessly interact with Dify's AI services by translating between their respective API formats.

## Architecture Overview

This application follows a clean, feature-based architecture pattern implemented in Rust, utilizing Actix-web for high-performance HTTP handling. Here's a detailed breakdown of the system:

### System Architecture Diagram

```mermaid
graph TD
    Client[OpenAI Client] -->|OpenAI Format Request| Proxy[Proxy Server]
    Proxy -->|Transform Request| DifyAPI[Dify API]
    DifyAPI -->|Dify Response| Proxy
    Proxy -->|Transform Response| Client
    
    subgraph Proxy Server
        Handler[Chat Completion Handler]
        Transform[Request/Response Transformer]
        Error[Error Handler]
        Stream[Stream Processor]
        
        Handler --> Transform
        Transform --> Stream
        Handler --> Error
    end
```

### Component Model

```mermaid
classDiagram
    class OpenAIRequest {
        +Vec~OpenAIMessage~ messages
        +Option~Vec~Tool~~ tools
        +Option~bool~ stream
        +Option~f32~ temperature
        +Option~f32~ top_p
        +Option~u32~ max_tokens
        +Option~String~ model
        +Option~String~ user
    }
    
    class DifyRequest {
        +Value inputs
        +Vec~String~ query
        +String response_mode
        +String user
        +Option~f32~ temperature
        +Option~f32~ top_p
        +Option~u32~ max_tokens
        +Option~Vec~Tool~~ tools
    }
    
    class OpenAIResponse {
        +String id
        +String object
        +u64 created
        +String model
        +Vec~OpenAIChoice~ choices
        +Option~Usage~ usage
    }
    
    class DifyResponse {
        +String event
        +String task_id
        +String conversation_id
        +String message_id
        +u64 created_at
        +String answer
        +Option~Vec~ToolCall~~ tool_calls
        +Option~Vec~File~~ files
    }
    
    OpenAIRequest --> DifyRequest : transforms to
    DifyResponse --> OpenAIResponse : transforms to

```

## Technical Architecture

### Core Components

1. **Request Handler (`chat_completion.rs`)**
   - Handles incoming OpenAI-format requests
   - Manages authentication and request validation
   - Coordinates request/response transformation
   - Supports both streaming and blocking responses

2. **Type System (`types.rs`)**
   - Defines the complete type hierarchy for both OpenAI and Dify formats
   - Implements serialization/deserialization using Serde
   - Provides robust error handling and type safety

3. **Application State (`app_state.rs`)**
   - Manages application configuration
   - Handles environment variable management
   - Provides shared state across requests

### Key Features

1. **Format Translation**
   - Bidirectional translation between OpenAI and Dify formats
   - Preserves all essential request/response parameters
   - Handles complex message structures and tool calls

2. **Streaming Support**
   - Real-time streaming of responses
   - Efficient chunk processing and transformation
   - Maintains connection stability

3. **Error Handling**
   - Comprehensive error type system
   - Detailed error messages and appropriate status codes
   - Graceful failure handling

### Performance Characteristics

- Built on Actix-web for high-performance async I/O
- Efficient memory usage through stream processing
- Minimal overhead in request/response transformation
- Configurable timeouts and connection management

## Configuration

The server requires the following environment variables:

- `DIFY_API_URL`: The URL of your Dify API endpoint
- `HOST`: Server host (defaults to "0.0.0.0")
- `PORT`: Server port (defaults to "8223")

Optional features can be enabled via Cargo features:
- `logging`: Enables detailed logging (enabled by default)

## Usage

1. Clone the repository
2. Configure environment variables (create a `.env` file)
3. Run the server:
   ```bash
   cargo run
   ```

### Example Request

```bash
curl http://localhost:8223/v1/chat/completions \
-H "Content-Type: application/json" \
-H "Authorization: Bearer your_dify_api_key_here" \
-d '{
  "model": "gpt-3.5-turbo",
  "messages": [{"role": "user", "content": "Hello!"}],
  "stream": true
}'
```

## Design Assessment

### Strengths

1. **Clean Architecture**
   - Clear separation of concerns
   - Feature-based organization
   - Strong type safety

2. **Robust Error Handling**
   - Comprehensive error types
   - Detailed error messages
   - Proper status code mapping

3. **Efficient Processing**
   - Stream-based processing
   - Minimal memory overhead
   - Async/await throughout

### Areas for Improvement

1. **Monitoring & Metrics**
   - Could add telemetry
   - Performance monitoring
   - Usage statistics

2. **Caching**
   - Request caching
   - Response caching
   - Rate limiting

3. **Documentation**
   - API documentation
   - More code comments
   - Integration examples

## Security Considerations

- API key validation
- CORS configuration
- Request validation
- Error message sanitization
- Timeout configuration

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

### MIT License Summary

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

- The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
- The software is provided "as is", without warranty of any kind.

Copyright (c) 2024 Garrett Quinn Adonis
