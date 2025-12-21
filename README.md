# opencode-share Rust

Rust implementation of the Opencode opencode-share package using Axum, SQLx, and SQLite.

## Features

- **High Performance**: Built with Rust and Axum for maximum performance
- **Type Safety**: Leverages Rust's type system for compile-time guarantees
- **Event Sourcing**: Implements efficient event-driven data synchronization
- **SQLite Database**: Lightweight, self-contained database with migrations
- **RESTful API**: Full REST API with proper error handling
- **CORS Support**: Cross-origin requests enabled
- **Structured Logging**: Comprehensive logging with tracing

## Architecture

- **Web Framework**: Axum
- **Database**: SQLite with SQLx (async, compile-time checked queries)
- **Event Sourcing**: Compaction strategy for efficient data storage
- **Static Files**: Built-in static file serving
- **Error Handling**: Comprehensive error types and HTTP status mapping

## Quick Start

### Prerequisites

- Rust 1.70+ 
- SQLite3

### Development

```bash
# Copy environment variables
cp .env.example .env

# Start development server
./dev.sh
```

The server will start at `http://localhost:3000`

### Production

```bash
# Build for production
./build.sh

# Run production binary
./target/release/opencode-share
```

## API Endpoints

### Create Share
```http
POST /api/share
Content-Type: application/json

{
  "sessionID": "your-session-id"
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
      "data": { ... }
    }
  ]
}
```

### Get Share Data
```http
GET /api/share/{shareID}/data
```

### Remove Share
```http
DELETE /api/share/{shareID}
Content-Type: application/json

{
  "secret": "share-secret"
}
```

### Share Page
```http
GET /share/{shareID}
```

## Database Schema

The system uses three main tables:

- `shares`: Share metadata (id, secret, session_id, created_at)
- `share_events`: Event log for data synchronization
- `share_compactions`: Compacted data for efficient retrieval

## Environment Variables

- `DATABASE_URL`: SQLite database path (default: `./opencode-share.db`)
- `RUST_LOG`: Log level (default: `opencode_share=debug,tower_http=debug`)
- `PORT`: Server port (default: `3000`)
- `HOST`: Server host (default: `0.0.0.0`)

## Migration from TypeScript

This implementation maintains full API compatibility with the original TypeScript version:

- Same endpoint URLs and request/response formats
- Compatible data structures
- Same event sourcing logic
- Equivalent share page functionality

## Performance Benefits

- **Memory Safety**: No garbage collection pauses
- **Zero-Cost Abstractions**: Compile-time optimizations
- **Async Runtime**: Efficient concurrent request handling
- **SQLite**: Fast embedded database without network overhead

## Development

### Project Structure

```
src/
├── main.rs              # Application entry point
├── core/
│   ├── mod.rs
│   └── share.rs         # Share business logic
├── database/
│   └── mod.rs           # Database setup and utilities
├── models.rs            # Data models and types
└── routes/
    ├── mod.rs
    ├── api.rs           # REST API routes
    └── share.rs         # Share page routes
static/                  # Static files (CSS, JS, images)
migrations/              # Database migrations
```

### Adding New Features

1. Define data types in `models.rs`
2. Implement business logic in `core/`
3. Add routes in `routes/`
4. Run migrations if needed

### Testing

```bash
# Run tests
cargo test

# Run with database
cargo test --features test-db
```

## License

Same as the original project.