#!/bin/bash

curl -N -X POST "http://localhost:8223/v1/chat/completions" \
-H "Content-Type: application/json" \
-H "Authorization: Bearer app-hYS1WT3xKqHdcFjfBUatLlUK" \
-d '{
  "model": "gpt-3.5-turbo",
  "messages": [
    {"role": "system", "content": "You are a helpful assistant."},
    {"role": "user", "content": "Tell me about reveles."}
  ],
  "max_tokens": 1024,
  "temperature": 0.1,
  "stream": true
}'