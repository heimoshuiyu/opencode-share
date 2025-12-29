# opencode-share Rust

Rust implementation of the Opencode share service for sharing AI coding agent sessions. Built with Axum, SQLx, and PostgreSQL for maximum performance and type safety.

## 🚀 Features

### Core Functionality
- **High Performance**: Built with Rust and Axum async framework for maximum throughput
- **Type Safety**: Compile-time checked SQL queries and strongly typed data structures
- **Event Sourcing**: Efficient event-driven data synchronization with JSONB storage
- **Secret-based Authentication**: Secure sharing without user account management
- **Rich Share Page**: Interactive UI for viewing AI coding sessions

### Frontend Features
- **🔧 Tool Call Visualization**: Beautiful cards showing bash commands, file operations, grep, etc.
- **📦 Collapsible Reasoning Blocks**: Display AI "thinking" with expand/collapse functionality
- **🔢 Step Markers**: Visual indicators for reasoning steps with token usage tracking
- **📝 Split Diff Viewer**: Side-by-side file change visualization with synchronized scrolling
- **🎨 Syntax Highlighting**: Code blocks with monospace fonts and proper styling
- **📊 Token Usage Tracking**: Display message and session token consumption

### Production Ready
- **Docker Support**: Multi-stage builds for optimized container images
- **Structured Logging**: Comprehensive request/response logging with emoji indicators
- **CORS Support**: Full cross-origin support for frontend integration
- **Environment Configuration**: Flexible configuration via environment variables
- **Client IP Extraction**: Proper handling of reverse proxy headers

## 📋 Architecture

### Technology Stack

| Component | Technology | Version |
|-----------|-----------|---------|
| **Language** | Rust | 2021 edition |
| **Web Framework** | Axum | 0.7 |
| **Runtime** | Tokio | 1.0 (async) |
| **Database** | PostgreSQL | (via SQLx) |
| **ORM** | SQLx | 0.7 (compile-time checked) |
| **Serialization** | Serde | 1.0 |
| **Frontend** | Vanilla ES6+ JavaScript + CSS3 |

### Project Structure

```
opencode-share/
├── src/
│   ├── main.rs              # Application entry point (84 lines)
│   ├── models.rs            # Data models and ShareData enum (76 lines)
│   ├── middleware.rs        # HTTP request logging middleware (119 lines)
│   ├── core/
│   │   ├── mod.rs           # Core module definition
│   │   └── share.rs         # Business logic for share operations (178 lines)
│   ├── database/
│   │   └── mod.rs           # PostgreSQL pool setup (13 lines)
│   └── routes/
│       ├── mod.rs           # Route module exports
│       ├── api.rs           # REST API endpoints (183 lines)
│       └── share.rs         # Share page rendering (82 lines)
├── static/
│   ├── share.js            # Client-side renderer (617 lines)
│   ├── share.css           # Styling (978 lines)
│   └── favicon/manifest    # PWA assets
├── templates/
│   └── share.html          # HTML template
├── migrations/
│   └── 001_initial.sql     # Database schema
├── Cargo.toml               # Rust dependencies
├── Dockerfile              # Multi-stage container build
├── docker-compose.yaml     # Orchestration with PostgreSQL
└── README.md
```

### Design Patterns

- **Layered Architecture**: Routes → Service → Data layers
- **Repository Pattern**: `ShareService` abstracts database operations
- **Event Sourcing**: Data stored as events in JSONB format
- **State Pattern**: `AppState` holds shared database pool
- **Tagged Unions**: `ShareData` enum with serde for type-safe event handling

## 🛠️ Quick Start

### Prerequisites

- Rust 1.70+
- PostgreSQL 12+
- Docker (optional, for containerized deployment)

### Using Docker Compose (Recommended)

```bash
# Start the service with PostgreSQL
docker-compose up -d

# View logs
docker-compose logs -f

# Stop the service
docker-compose down
```

The server will start at `http://localhost:3006`

### Manual Development

```bash
# Set environment variables
export DATABASE_URL="postgres://postgres:password@localhost:5432/opencode_share"
export RUST_LOG="opencode_share=debug,tower_http=debug"
export PORT=3006

# Run the server
cargo run
```

### Production Build

```bash
# Build the release binary
cargo build --release

# Run the binary
./target/release/opencode-share
```

## 📡 API Endpoints

### Create Share

```http
POST /api/share
Content-Type: application/json

{
  "sessionID": "your-session-id"
}
```

**Response:**
```json
{
  "id": "share-id",
  "secret": "uuid-secret",
  "url": "http://localhost:3006/share/share-id"
}
```

### Sync Share Data

```http
POST /api/share/{shareID}/sync
Content-Type: application/json

{
  "secret": "share-secret",
  "data": [
    {
      "type": "session",
      "data": {
        "id": "session-id",
        "title": "Session Title",
        "status": "active"
      }
    },
    {
      "type": "message",
      "data": {
        "id": "message-id",
        "role": "assistant",
        "content": "Response text"
      }
    }
  ]
}
```

### Get Share Data

```http
GET /api/share/{shareID}/data
```

**Response:** Array of `ShareData` objects

### Remove Share

```http
DELETE /api/share/{shareID}
Content-Type: application/json

{
  "secret": "share-secret"
}
```

### View Share Page

```http
GET /share/{shareID}
```

## 🗄️ Database Schema

### Shares Table

```sql
CREATE TABLE shares (
    id TEXT PRIMARY KEY,
    secret TEXT NOT NULL,
    session_id TEXT NOT NULL,
    data JSONB DEFAULT '[]',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
```

### Indexes

```sql
CREATE INDEX idx_shares_session_id ON shares(session_id);
CREATE INDEX idx_shares_created_at ON shares(created_at);
CREATE INDEX idx_shares_updated_at ON shares(updated_at);
CREATE INDEX idx_shares_data_gin ON shares USING GIN (data);
```

### ShareData Types

Data is stored as a discriminated union in JSONB:

- `session`: Session metadata and status
- `message`: User/assistant messages
- `part`: Message parts (text, code blocks, tool outputs)
- `session_diff`: File changes made during session
- `model`: Model information (provider, name)

## ⚙️ Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `DATABASE_URL` | PostgreSQL connection string | - |
| `RUST_LOG` | Log level | `opencode_share=info,tower_http=info` |
| `PORT` | Server port | `3006` |
| `HOST` | Server host | `0.0.0.0` |

### Docker Compose Configuration

```yaml
services:
  postgres:
    image: postgres:15
    environment:
      POSTGRES_PASSWORD: password
      POSTGRES_DB: opencode_share
    volumes:
      - postgres_data:/var/lib/postgresql/data
    ports:
      - "5432:5432"

  opencode-share:
    build: .
    environment:
      DATABASE_URL: postgres://postgres:password@postgres:5432/opencode_share
      RUST_LOG: opencode_share=debug,tower_http=debug
      PORT: 3006
    ports:
      - "3006:3006"
    depends_on:
      - postgres
    restart: unless-stopped

volumes:
  postgres_data:
```

## 🔌 Integrating with OpenCode

### Step 1: Configure OpenCode

Create or update `opencode.json` in your OpenCode project:

```json
{
  "$schema": "https://opencode.ai/config.json",
  "enterprise": {
    "url": "http://localhost:3006"
  },
  "share": "manual"
}
```

### Step 2: Share a Session

In the OpenCode TUI/CLI:

```bash
/share
```

This will create a share and provide you with a URL to view the session.

### Step 3: Access Shared Session

Navigate to the provided URL in your browser to view the shared session.

## 🎨 Customization

### Customizing the Share Page

The share page can be customized by modifying:

- **HTML Template**: `templates/share.html`
- **CSS Styling**: `static/share.css`
- **JavaScript Renderer**: `static/share.js`

### Adding Custom Endpoints

1. Define your route in `src/routes/api.rs`:

```rust
pub fn api_routes() -> Router<AppState> {
    Router::new()
        .route("/api/share/:id/custom", post(custom_endpoint))
        .with_state(app_state)
}

async fn custom_endpoint(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, AppError> {
    // Your custom logic
    Ok(Json(json!({ "result": "success" })))
}
```

2. Add business logic in `src/core/share.rs`:

```rust
impl ShareService {
    pub async fn custom_operation(
        &self,
        id: &str,
    ) -> Result<ShareData, anyhow::Error> {
        // Implementation
    }
}
```

### Custom Data Types

Add new data types to the `ShareData` enum in `src/models.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ShareData {
    #[serde(rename = "session")]
    Session { data: Value },

    #[serde(rename = "custom")]
    Custom { data: YourCustomType },
}
```

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run tests with database
cargo test --features test-db

# Run specific test
cargo test test_create_share
```

## 📊 Performance Benefits

- **Memory Safety**: No garbage collection pauses
- **Zero-Cost Abstractions**: Compile-time optimizations
- **Async Runtime**: Efficient concurrent request handling with Tokio
- **PostgreSQL + JSONB**: Fast queries with flexible schema
- **SQLx**: Compile-time checked SQL queries prevent runtime errors

## 🔐 Security

- **Secret-based Authentication**: Each share has a unique secret key
- **SQL Injection Prevention**: Parameterized queries via SQLx
- **Input Validation**: Type checking through Serde
- **CORS**: Configurable cross-origin resource sharing

## 📈 Migration from TypeScript

This implementation maintains full API compatibility with the original TypeScript version:

- ✅ Same endpoint URLs and request/response formats
- ✅ Compatible data structures
- ✅ Same event sourcing logic
- ✅ Equivalent share page functionality

## 🤝 Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📝 Development Tips

### Code Organization

- **Models**: Define data types in `src/models.rs`
- **Core Logic**: Implement business logic in `src/core/share.rs`
- **Routes**: Add API endpoints in `src/routes/api.rs`
- **Database**: Add migrations in `migrations/`

### Logging

The service uses structured logging with tracing:

```rust
use tracing::{info, warn, error};

info!("share_created", id = %share_id);
warn!("sync_failed", share_id = %id, error = %err);
```

### Error Handling

Use `anyhow::Error` for error propagation:

```rust
pub async fn create_share(&self, session_id: &str) -> Result<ShareInfo, anyhow::Error> {
    let share = sqlx::query_as::<_, ShareInfo>(...)
        .fetch_one(&self.pool)
        .await?;
    Ok(share)
}
```

## 📄 License

Same as the original Opencode project.

## 🔗 Links

- [OpenCode](https://opencode.ai)
- [Axum Documentation](https://docs.rs/axum)
- [SQLx Documentation](https://docs.rs/sqlx)
- [Tokio Documentation](https://tokio.rs)
