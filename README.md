# efficeintnlp

Rust server for routing natural-language requests into HTTP calls and local commands.

## Share With Friends

The easiest path is:

1. Push this repo to GitHub.
2. Create and push a tag like `v0.1.0`.
3. Let `.github/workflows/release.yml` build and publish the tagged release assets.
4. Tell friends to download the archive for their OS from GitHub Releases.
5. Run the binary with `data.json` in the same directory.

`workflow_dispatch` runs still produce GitHub Actions artifacts, but the installers use GitHub Release assets under `releases/latest/download/...`, so they only work after a tagged release has published successfully.

The release archives are slim by default and do not include `models/`.
If model files are missing, the server downloads the default models on first startup.

One-line installers:

Linux/macOS:

```bash
curl -fsSL https://raw.githubusercontent.com/OWNER/REPO/main/install.sh | bash -s -- OWNER/REPO latest default
```

Windows PowerShell:

```powershell
iwr https://raw.githubusercontent.com/OWNER/REPO/main/install.ps1 -useb -OutFile install.ps1; ./install.ps1 OWNER/REPO latest default
```

You can also install a specific release tag and variant:

```bash
curl -fsSL https://raw.githubusercontent.com/OWNER/REPO/main/install.sh | bash -s -- OWNER/REPO v0.1.0 fire-and-forget-ui
```

Supported variants:

- `default`
- `fire-and-forget`
- `default-ui`
- `fire-and-forget-ui`

## Local Run

```bash
cargo run
```

Fire-and-forget mode:

```bash
cargo run --features fire-and-forget
```

UI mode:

```bash
cargo run --features ui
```

UI + fire-and-forget:

```bash
cargo run --features "ui fire-and-forget"
```

## Docker

Default Docker builds are slim and do not bundle model files:

```bash
docker build -t efficeintnlp .
docker run --rm -p 8787:8787 efficeintnlp
```

The container creates `/app/models/...` directories and downloads configured models on first startup if they are missing.

For persistent model caching in Docker, mount a host or named volume:

```bash
docker run --rm -p 8787:8787 -v efficeintnlp-models:/app/models efficeintnlp
```

For release-based container publishing, GitHub Actions builds the Linux binary first and then uses [Dockerfile.runtime](/home/shreyas/Desktop/quicktasks%20(copy)/S/3/Dockerfile.runtime) to build a runtime image from that artifact instead of recompiling inside Docker.

Published container tags:

- `ghcr.io/OWNER/REPO:latest`
- `ghcr.io/OWNER/REPO:v0.1.0`
- `ghcr.io/OWNER/REPO:v0.1.0-fire-and-forget`
- `ghcr.io/OWNER/REPO:v0.1.0-default-ui`
- `ghcr.io/OWNER/REPO:v0.1.0-fire-and-forget-ui`

## Cloud Deployment

For small deployments, first-run downloads are fine.

For larger cloud deployments, prefer one of:

- mount `/app/models` from persistent storage
- pre-populate model files during deploy
- point `data.json` at local paths already present on the machine

Avoid relying on every replica downloading multi-GB models during cold start.

Cloud-friendly env vars:

- `EFFICEINTNLP_CONFIG`
  Path to the active config file. Default: `data.json`
- `EFFICEINTNLP_MODEL_DIR`
  Base directory for downloaded/default model cache. Default: `models`
- `EFFICEINTNLP_GTE_MODEL_PATH`
- `EFFICEINTNLP_GTE_TOKENIZER_PATH`
- `EFFICEINTNLP_GLINER_MODEL_PATH`
- `EFFICEINTNLP_GLINER_TOKENIZER_PATH`

That lets users:

- mount their own `data.json`
- mount pre-provisioned models
- point the server at custom local model files
- keep default fallback downloads when no local model files are present

## Custom Models

`data.json` supports custom model sources in `components`:

- local file paths
- remote URLs

That means users can supply their own models without rebuilding the binary.

Example shape:

```json
{
  "components": {
    "functionMapper": {
      "modelUrl": "/opt/models/gte/model.onnx",
      "tokenizerUrl": "/opt/models/gte/tokenizer.json"
    },
    "entityRecognizer": {
      "modelUrl": "https://example.com/gliner/model.onnx",
      "tokenizerUrl": "https://example.com/gliner/tokenizer.json"
    }
  }
}
```

## API Endpoints

The server exposes several HTTP API endpoints for interaction. All examples assume the server is running on `http://localhost:8787`.

### Main Query Endpoint

**Method:** `POST`
**Path:** `/query`
**Description:** Sends a natural language query to the server for processing.

**Request:**
*   **Headers:**
    *   `Content-Type: application/json`
*   **Body:** A JSON object with the following field:
    *   `query` (`string`, required): The natural language query string.

**Example `curl` Request:**

```bash
curl -X POST http://localhost:8787/query \
    -H "Content-Type: application/json" \
    -d '{"query": "your query"}'
```

**Expected Response:**
A JSON object containing the status, the function that was executed, and the result payload. The `result` field can vary based on the function's output.

```json
{
  "status": "success",
  "function": "executed_function_name",
  "result": {
    "Json": {
      "key": "value"
    }
  }
}
```
*Note: The `result` field can also be `String`, `File`, `Files`, or `Binary` depending on the function's output.*