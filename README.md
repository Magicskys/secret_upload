# secret_upload

A simple, lightweight, and secure file upload server written in Rust using the Axum framework.

## Motivation

Modern servers often sit behind SSH jump hosts or complex VPNs, making simple file transfers via `scp` or `sftp` a multi-step chore. **secret_upload** provides a "run-anywhere" solution to bridge this gap. 

Instead of configuring tunnels or multi-hop SCP commands, just start this binary on your remote server. You can then instantly upload files from your local machine via a web browser or a single `curl` command, making it the perfect tool for quick deployments and log collection in restricted environments.

## Features

- **Password Protected**: Only users with the correct password can upload files.
- **Embedded UI**: A clean, built-in HTML upload page—no external files required.
- **Large File Support**: Memory-efficient streaming uploads (supports GB-sized files).
- **Client-Side Validation**: Visual hints and instant file-size checks in the browser.
- **CLI Support**: Easily upload files using `curl`.
- **Customizable**: Set the bind IP, port, password, max size, and timeouts via command-line arguments.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable version recommended)

## Installation

1. Clone the repository or download the source code.
2. Build the project:
   ```bash
   cargo build --release
   ```

## Usage

Run the server by providing a mandatory password:

```bash
# Basic usage (2GB limit, no timeout)
./target/release/secret_upload --password my_secret_password

# Custom limit (e.g., 5GB) and timeout (e.g., 30 mins)
./target/release/secret_upload --password my_secret_password --max-size 5G --timeout 1800
```

### Argument Details

- `--password <PASSWORD>`: **(Required)** The password required for uploading files.
- `-i, --ip <IP>`: The IP address to bind the server to (default: `0.0.0.0`).
- `-p, --port <PORT>`: The port to listen on (default: `43000`).
- `--max-size <SIZE>`: The maximum upload size (e.g., `2G`, `500M`, `100MB`) (default: `2G`).
- `--timeout <SECONDS>`: Request timeout in seconds (default: none).

## How to Upload

### 1. Via Browser
Navigate to `http://localhost:43000` (or your configured IP/Port), enter the password, select a file, and click **Upload**. The page will show the maximum allowed size and prevent you from selecting files that are too large.

### 2. Via cURL
You can use the following command for quick uploads:
```bash
curl -F "password=YOUR_PASSWORD" -F "file=@/path/to/your/file" http://localhost:43000/upload
```

## Security Notice

- **Streaming**: Files are streamed directly to the working directory to avoid excessive memory usage.
- **Limits**: Both the server (via `tower-http`) and the browser (via JS) enforce the configured file size limit.

## License

This project is open-source and available under the MIT License.
