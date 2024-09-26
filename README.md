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
- `DIFY_API_KEY`: Your Dify API key for the Authorization header

You can set these in a `.env` file or in your environment before running the server.

Note: The `DIFY_API_KEY` should be passed in the Authorization header when making requests to the Dify API.

## Note

This proxy is designed to work with Dify's API. Ensure you have the necessary permissions and API access from Dify before using this proxy.

## Image Support

The server uses IPFS to store images. The IPFS upload URL is set in the `.env` file.

The image storage directory is set in the `.env` file.



Certainly! Here's a detailed markdown table describing each parameter available via HTTP header for the image generation API:

| HTTP Header | Parameter Name | Purpose | Acceptable Values |
|-------------|----------------|---------|-------------------|
| X-Output-Format | output_format | Specifies the desired output image format | "jpg", "png", "webp" |
| X-Replicate-Scheduler | scheduler | Defines the sampling method used in the diffusion process | "DDIM", "DPMSolverMultistep", "K_EULER", "K_EULER_ANCESTRAL", "PNDM" |
| X-Replicate-Num-Inference-Steps | num_inference_steps | Controls the number of denoising steps in the diffusion process | Integer between 1 and 500 (typical range: 20-100) |
| X-Replicate-Guidance-Scale | guidance_scale | Determines how closely the image should adhere to the prompt | Float between 1 and 20 (typical range: 5-15) |
| X-Replicate-Seed | seed | Sets a specific seed for reproducible results | Integer (0 or greater) |
| X-Replicate-Negative-Prompt | negative_prompt | Specifies what the model should avoid in the image | String |
| X-Replicate-Prompt-Strength | prompt_strength | Controls the balance between the input image and the prompt in img2img | Float between 0 and 1 |
| X-Replicate-Num-Outputs | num_outputs | Specifies the number of images to generate | Integer between 1 and 4 |
| X-Replicate-Safety-Checker | safety_checker | Enables or disables the safety filter | "yes" or "no" |
| X-Replicate-Enhance-Prompt | enhance_prompt | Enables AI-assisted prompt enhancement | "yes" or "no" |
| X-Replicate-Upscale | upscale | Requests image upscaling after generation | "yes" or "no" |
| X-Replicate-Upscale-Factor | upscale_factor | Specifies the factor by which to upscale the image | Integer (typically 2 or 4) |
| X-Replicate-Style-Preset | style_preset | Applies a predefined style to the generated image | String (e.g., "anime", "photographic", "digital-art") |
| X-Replicate-Init-Image | init_image | URL of an initial image for img2img or inpainting | Valid URL string |
| X-Replicate-Mask | mask | URL of a mask image for inpainting | Valid URL string |

Please note:

1. The actual availability and behavior of these parameters may vary depending on the specific model version and configuration used in the Replicate API.

2. Some parameters may be mutually exclusive or only applicable in certain contexts (e.g., `init_image` and `mask` are only used for img2img or inpainting tasks).

3. The acceptable values provided are general guidelines. Specific models might have different ranges or options.

4. Always refer to the most up-to-date Replicate API documentation for the exact parameters supported by the model you're using, as they may change or be updated over time.

5. When implementing these headers in your API, make sure to validate the input values to ensure they fall within acceptable ranges and formats.

This table provides a comprehensive overview of the parameters that can be controlled via HTTP headers in your image generation API, allowing for fine-grained control over the generation process while maintaining compatibility with the OpenAI-style request body.