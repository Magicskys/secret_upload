use axum::{
    extract::{DefaultBodyLimit, State},
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use clap::Parser;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None, disable_help_flag = true)]
struct Args {
    /// Password for uploading files
    #[arg(short = 'h', long)]
    password: String,

    /// IP address to bind to
    #[arg(short, long, default_value = "0.0.0.0")]
    ip: String,

    /// Port to listen on
    #[arg(short, long, default_value_t = 43000)]
    port: u16,

    /// Maximum upload size (e.g., 2G, 500M, 100MB)
    #[arg(long, default_value = "2G", value_parser = parse_size)]
    max_size: u64,

    /// Timeout in seconds (default: no limit)
    #[arg(long)]
    timeout: Option<u64>,

    /// Print help
    #[arg(long, action = clap::ArgAction::Help)]
    help: Option<bool>,
}

fn parse_size(s: &str) -> Result<u64, String> {
    let s = s.trim().to_lowercase();
    let (num_part, unit_part) = s
        .find(|c: char| !c.is_numeric())
        .map(|idx| s.split_at(idx))
        .unwrap_or((&s, ""));

    let num: u64 = num_part
        .parse()
        .map_err(|_| format!("Invalid number: {}", num_part))?;

    match unit_part {
        "" | "b" => Ok(num),
        "k" | "kb" => Ok(num * 1024),
        "m" | "mb" => Ok(num * 1024 * 1024),
        "g" | "gb" => Ok(num * 1024 * 1024 * 1024),
        "t" | "tb" => Ok(num * 1024 * 1024 * 1024 * 1024),
        _ => Err(format!("Invalid unit: {}", unit_part)),
    }
}

struct AppState {
    password: String,
    max_size: u64,
}

async fn get_local_ip() -> String {
    local_ip_address::local_ip()
        .map(|ip| ip.to_string())
        .unwrap_or_else(|_| "127.0.0.1".to_string())
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let shared_state = Arc::new(AppState {
        password: args.password.clone(),
        max_size: args.max_size,
    });

    let mut app = Router::new()
        .route("/", get(index))
        .route("/upload", post(upload))
        .layer(DefaultBodyLimit::disable())
        .layer(RequestBodyLimitLayer::new(args.max_size as usize));

    if let Some(timeout_secs) = args.timeout {
        app = app.layer(TimeoutLayer::new(Duration::from_secs(timeout_secs)));
    }

    let app = app.with_state(shared_state);

    let addr_str = format!("{}:{}", args.ip, args.port);
    let addr: SocketAddr = addr_str.parse().expect("Invalid IP or Port");

    let display_ip = if args.ip == "0.0.0.0" {
        get_local_ip().await
    } else {
        args.ip.clone()
    };

    println!("Listening on http://{}\n", addr);
    println!("Max upload size: {} bytes ({:.2} GB)", args.max_size, args.max_size as f64 / 1024.0 / 1024.0 / 1024.0);
    if let Some(t) = args.timeout {
        println!("Timeout: {} seconds", t);
    } else {
        println!("Timeout: No limit");
    }
    
    println!("\nShortcut upload command:");
    println!(
        "  curl -F \"password={}\" -F \"file=@/path/to/file\" http://{}:{}/upload",
        args.password, display_ip, args.port
    );

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn index(State(state): State<Arc<AppState>>) -> Html<String> {
    let max_gb = state.max_size as f64 / 1024.0 / 1024.0 / 1024.0;
    Html(format!(r#"
        <!DOCTYPE html>
        <html>
        <head>
            <meta charset="UTF-8">
            <title>Secret File Upload</title>
            <style>
                body {{ font-family: sans-serif; display: flex; justify-content: center; align-items: center; height: 100vh; margin: 0; background-color: #f0f2f5; }}
                .upload-card {{ background: white; padding: 2rem; border-radius: 8px; box-shadow: 0 4px 6px rgba(0,0,0,0.1); width: 350px; }}
                h2 {{ margin-top: 0; color: #333; }}
                .form-group {{ margin-bottom: 1rem; }}
                label {{ display: block; margin-bottom: .5rem; font-size: 0.9rem; color: #666; }}
                input[type="text"], input[type="password"], input[type="file"] {{ width: 100%; padding: 0.5rem; box-sizing: border-box; border: 1px solid #ccc; border-radius: 4px; }}
                button {{ width: 100%; padding: 0.75rem; background-color: #007bff; color: white; border: none; border-radius: 4px; cursor: pointer; font-weight: bold; }}
                button:hover {{ background-color: #0056b3; }}
                .hint {{ font-size: 0.8rem; color: #888; margin-top: 0.25rem; }}
                .error {{ color: #dc3545; font-size: 0.8rem; margin-top: 0.5rem; display: none; }}
            </style>
        </head>
        <body>
            <div class="upload-card">
                <h2>Upload File</h2>
                <form action="/upload" method="post" enctype="multipart/form-data" id="uploadForm">
                    <div class="form-group">
                        <label for="password">Password</label>
                        <input type="password" name="password" id="password" required>
                    </div>
                    <div class="form-group">
                        <label for="file">File</label>
                        <input type="file" name="file" id="file" required>
                        <div class="hint">Max size: {:.2} GB</div>
                        <div id="sizeError" class="error">File size exceeds the limit.</div>
                    </div>
                    <button type="submit" id="submitBtn">Upload</button>
                </form>
            </div>
            <script>
                const form = document.getElementById('uploadForm');
                const fileInput = document.getElementById('file');
                const sizeError = document.getElementById('sizeError');
                const maxSize = {};

                fileInput.addEventListener('change', function() {{
                    if (this.files[0] && this.files[0].size > maxSize) {{
                        sizeError.style.display = 'block';
                        this.value = '';
                    }} else {{
                        sizeError.style.display = 'none';
                    }}
                }});

                form.addEventListener('submit', function(e) {{
                    if (fileInput.files[0] && fileInput.files[0].size > maxSize) {{
                        e.preventDefault();
                        alert('File too large!');
                    }}
                }});
            </script>
        </body>
        </html>
    "#, max_gb, state.max_size))
}

async fn upload(
    State(state): State<Arc<AppState>>,
    mut multipart: axum::extract::Multipart,
) -> Result<Html<String>, String> {
    let mut password_match = false;
    let mut filename_uploaded: Option<String> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| e.to_string())? {
        let name = field.name().unwrap_or_default().to_string();

        if name == "password" {
            let data = field.text().await.map_err(|e| e.to_string())?;
            if data == state.password {
                password_match = true;
            }
        } else if name == "file" {
            if !password_match {
                return Err("Incorrect password or password not provided before file".to_string());
            }

            let raw_filename = field.file_name().unwrap_or("uploaded_file").to_string();
            let safe_filename = Path::new(&raw_filename)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("uploaded_file")
                .to_string();

            let mut file = File::create(&safe_filename).await.map_err(|e| e.to_string())?;
            let mut field = field;
            while let Some(chunk) = field.chunk().await.map_err(|e| e.to_string())? {
                file.write_all(&chunk).await.map_err(|e| e.to_string())?;
            }
            filename_uploaded = Some(safe_filename);
        }
    }

    if !password_match {
        return Err("Incorrect password".to_string());
    }

    if let Some(filename) = filename_uploaded {
        Ok(Html(format!(
            "<h1>File {} uploaded successfully!</h1><a href='/'>Back</a>",
            filename
        )))
    } else {
        Err("Missing file".to_string())
    }
}
