
 here is a focused analysis of **OnionShare** (functions only) and how it could integrate with AlLibrary.

---

# OnionShare – Functional Analysis & AlLibrary Integration

## 1. Architecture Overview

OnionShare is a Python app that runs a local HTTP server and exposes it as a Tor onion service. It has four modes:

| Mode | Purpose |
|------|---------|
| **Share** | Serve files/folders for download over Tor |
| **Receive** | Accept file uploads and text messages over Tor |
| **Website** | Serve static HTML/JS/CSS over Tor |
| **Chat** | Real-time chat over Tor (Flask-SocketIO) |

---

## 2. Core Components

### 2.1 Tor / Onion Layer (`onion.py`)

**Role:** Connect to Tor and create ephemeral onion services.

**Main functions:**

1. **`connect()`**
   - Connection types: `bundled`, `automatic`, `control_port`, `socket_file`
   - **Bundled:** Starts its own Tor process, writes `torrc`, uses `stem.Controller`
   - **Automatic:** Tries ports 9151, 9153, 9051 and common socket paths
   - **Custom:** Uses user-provided control port or socket path
   - Auth: cookie file or password
   - Optional bridges: obfs4, meek-azure, snowflake, moat, custom

2. **`start_onion_service(mode, mode_settings, port, await_publication)`**
   - Uses `stem.Controller.create_ephemeral_hidden_service()`
   - Maps Tor port 80 → local `port`
   - Supports:
     - **Public:** Anyone can access
     - **Stealth (client auth):** Only clients with private key can access
   - Key types: `ED25519-V3` (v3 onions)
   - Returns `{service_id}.onion`

3. **`stop_onion_service(mode_settings)`**
   - Calls `remove_ephemeral_hidden_service(service_id)`

4. **`get_tor_socks_port()`**
   - Returns `(address, port)` for SOCKS5 proxy

5. **`cleanup(stop_tor, wait)`**
   - Removes onion services, optionally stops Tor, waits for rendezvous circuits to close

**Dependencies:** `stem`, `psutil`, `nacl.public`, `packaging.version`

---

### 2.2 OnionShare Orchestrator (`onionshare.py`)

**Role:** Coordinates Tor and the web server.

- **`choose_port()`** – Picks a random port in 17600–17650
- **`start_onion_service(mode, mode_settings, await_publication)`** – Starts onion service via `Onion`
- **`stop_onion_service(mode_settings)`** – Stops onion service
- **`local_only`** – Skips Tor, uses `127.0.0.1:{port}` (dev)
- **`autostop_timer`** – Optional auto-shutdown after N seconds

---

### 2.3 Web Server (`web/web.py`)

**Role:** Flask app + Waitress (or Flask-SocketIO for chat).

**Stack:**

- Flask + Flask-Compress
- Waitress (production WSGI)
- Flask-SocketIO (chat only)
- Random `static_url_path` to avoid collisions with shared filenames

**Security headers:**

- `X-Frame-Options: DENY`
- `X-Content-Type-Options: nosniff`
- `Referrer-Policy: no-referrer`
- CSP (configurable in website mode)

**Lifecycle:**

- `start(port)` – Binds to `127.0.0.1` (or `0.0.0.0` on Whonix)
- `stop(port)` – Puts item in `stop_q`, triggers shutdown
- `waitress_custom_shutdown()` – Immediate shutdown

**Request queue:** `q` – Sends events (load, progress, cancel, etc.) to GUI/CLI.

---

### 2.4 Share Mode (`web/share_mode.py`)

**Role:** Serve files for download.

**Flow:**

1. **`set_file_info(filenames)`** – Builds file list, optionally zips
2. **Single file:** Gzip-compressed on demand if client supports it
3. **Multiple files/dirs:** Zipped via `ZipWriter`
4. **Routes:**
   - `GET /` – Directory listing or download page
   - `GET /<path>` – Directory listing or individual file
   - `GET /download` – Full zip or single file download

**Features:**

- HTTP Range requests (partial downloads)
- ETag, Last-Modified, 304 Not Modified
- Optional autostop after first download
- Optional individual file downloads (when autostop is off)
- Chunked streaming (100KB chunks)
- Progress events via `add_request()`

---

### 2.5 Receive Mode (`web/receive_mode.py`)

**Role:** Accept uploads and text messages.

**Flow:**

1. **`ReceiveModeRequest`** – Custom Flask Request subclass
2. **`ReceiveModeFile`** – File-like object that tracks write progress
3. **`ReceiveModeWSGIMiddleware`** – Injects `web` into `environ`

**Routes:**

- `GET /` – Upload form
- `POST /upload` – Handle multipart upload
- `POST /upload-ajax` – Same, JSON response

**Features:**

- Per-upload directory: `{data_dir}/{date}/{time}/`
- Text message saved as `{dir}-message.txt` (max 524288 chars)
- Optional webhook on successful upload
- Optional disable text or files
- Progress tracking via custom `write()` and `close()`

---

### 2.6 Chat Mode (`web/chat_mode.py`)

**Role:** Real-time chat over WebSockets.

**Features:**

- Username validation (ASCII letters/numbers, dash, underscore, space)
- Session-based usernames
- Flask-SocketIO for WebSocket events
- `connected_users` list

---

### 2.7 Common Utilities (`common.py`)

- **`get_available_port(min, max)`** – Random free port
- **`get_tor_paths()`** – Tor binary and pluggable transports (obfs4, snowflake, meek)
- **`build_password(word_count)`** – Random words from wordlist
- **`build_username(word_count)`** – Same for usernames
- **`random_string(num_bytes, output_len)`** – Base32 random string
- **`human_readable_filesize(b)`** – Human-readable size
- **`dir_size(path)`** – Total directory size

---

### 2.8 Censorship Circumvention (`censorship.py`)

- **`request_map(country)`** – Tor circumvention map
- **`request_builtin_bridges()`** – Bridges from Tor API
- Uses Tor SOCKS or Meek (domain fronting) if Tor is not available

---

## 3. Integration Options for AlLibrary

AlLibrary is Tauri/Rust + SolidJS. OnionShare is Python. Integration can be done in several ways.

### Option A: Subprocess Integration (Recommended)

Run OnionShare CLI as a subprocess and control it from Tauri.

**Flow:**

1. User chooses document(s) to share
2. Tauri invokes `onionshare-cli` with `--local-only` or Tor
3. Parse stdout for onion URL
4. Show URL (and optional QR) in UI
5. On stop, send SIGINT or call shutdown endpoint

**CLI usage:**

```bash
# Share mode
onionshare-cli /path/to/document.pdf

# Receive mode
onionshare-cli --receive --data-dir /path/to/save

# With options
onionshare-cli --no-autostop-sharing --public /path/to/folder
```

**Pros:** No Python in AlLibrary, uses existing OnionShare
**Cons:** Requires OnionShare installed; cross-platform packaging

---

### Option B: Port OnionShare Logic to Rust

Reimplement the core behavior in Rust.

**Components to implement:**

1. **Tor control** – Use `arti` or `tor-ctrl` (or `stem`-like API) for ephemeral hidden services
2. **HTTP server** – `axum` or `actix-web` for share/receive
3. **File handling** – Zip creation, streaming, range requests
4. **Upload handling** – Multipart parsing, progress tracking

**Pros:** Single binary, no Python
**Cons:** Significant effort; need to keep parity with OnionShare security and behavior

---

### Option C: OnionShare as a Library / HTTP API

If OnionShare exposed an API, AlLibrary could drive it over HTTP. It does not today; you would need to add a small REST/WebSocket layer in OnionShare.

---

## 4. Recommended Integration: Subprocess + Tauri

### 4.1 Tauri Commands

```rust
// Start share - returns onion URL when ready
#[tauri::command]
async fn start_onionshare_share(file_paths: Vec<String>, public: bool) -> Result<ShareResult, String>

// Stop share
#[tauri::command]
async fn stop_onionshare_share() -> Result<(), String>

// Start receive - returns onion URL
#[tauri::command]
async fn start_onionshare_receive(data_dir: String) -> Result<ShareResult, String>

// Stop receive
#[tauri::command]
async fn stop_onionshare_receive() -> Result<(), String>

// Check if OnionShare is available
#[tauri::command]
async fn is_onionshare_available() -> bool
```

### 4.2 Share Flow

1. User selects document(s) in AlLibrary
2. Call `start_onionshare_share(paths, public)`:
   - Spawn `onionshare-cli --local-only` or `onionshare-cli` (with Tor)
   - Store PID
   - Wait for "Give this address" in stdout
   - Parse `http://{id}.onion` or `127.0.0.1:{port}`
   - Return URL and auth string (if private)
3. UI shows URL and optional QR
4. On stop: `stop_onionshare_share()` sends SIGINT to process

### 4.3 Receive Flow

1. User selects folder to receive into
2. Call `start_onionshare_receive(data_dir)`:
   - Spawn `onionshare-cli --receive --data-dir {data_dir}`
   - Parse onion URL from stdout
3. UI shows URL and optional QR
4. On stop: `stop_onionshare_receive()` sends SIGINT

### 4.4 Tor Integration

**If AlLibrary already uses Tor:**

- Prefer `automatic` or `control_port` so OnionShare uses the same Tor
- Set `TOR_CONTROL_PORT` env var before spawning OnionShare
- Or use `--config` with a custom config file

**If Tor is bundled:**

- OnionShare can use `bundled` and start its own Tor
- Or AlLibrary starts Tor and OnionShare uses its control port

**If no Tor:**

- Use `--local-only` for local testing

---

## 5. API Surface Summary

| Function | Location | Purpose |
|----------|----------|---------|
| `Onion.connect()` | onion.py | Connect to Tor (bundled/automatic/custom) |
| `Onion.start_onion_service()` | onion.py | Create ephemeral onion service |
| `Onion.stop_onion_service()` | onion.py | Remove onion service |
| `Onion.get_tor_socks_port()` | onion.py | SOCKS proxy for Tor |
| `OnionShare.choose_port()` | onionshare.py | Pick random port |
| `OnionShare.start_onion_service()` | onionshare.py | Start onion |
| `Web.start(port)` | web.py | Start Flask/Waitress |
| `Web.stop(port)` | web.py | Stop server |
| `ShareModeWeb.set_file_info()` | share_mode.py | Prepare files for sharing |
| `ReceiveModeWeb` routes | receive_mode.py | Upload handling |
| `ModeSettings` | mode_settings.py | Per-mode config |

---

## 6. AlLibrary ↔ OnionShare Mapping

| AlLibrary Feature | OnionShare Equivalent |
|-------------------|------------------------|
| Share document via P2P | `share` mode + onion URL |
| Receive document | `receive` mode |
| TOR network | OnionShare Tor integration |
| `share_document_p2p` | Could spawn OnionShare share mode |
| `create_share_link` | OnionShare URL + optional client auth |

---

## 7. Practical Steps

1. **Detect OnionShare:** `which onionshare-cli` (Unix) or `where onionshare-cli` (Windows)
2. **Bundle or require:** Either bundle OnionShare in the build or document it as a dependency
3. **Subprocess:** Use `tokio::process::Command` to run `onionshare-cli` with the right args
4. **Parse stdout:** Regex or line-by-line parsing for `http://...` and `Private key: ...`
5. **UI:** Show URL, copy button, optional QR
6. **Stop:** Send SIGINT (Ctrl+C) or implement a shutdown endpoint in OnionShare

This approach keeps AlLibrary’s stack (Tauri/Rust/SolidJS) and adds OnionShare as a well-tested tool for anonymous, Tor-based sharing and receiving.
