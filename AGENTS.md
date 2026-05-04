# Developer Guide

mdow internals — for contributors, developers, and AI agents.

## Stack

- **Rust** — backend
- **Axum** — web framework
- **SQLite** — storage, via LiteFS
- **Fly.io** — deployment

## Local development

```bash
cargo build
cargo run
```

Runs at [http://localhost:8081](http://localhost:8081).

For hot-reload:

```bash
cargo watch -x run
```

## Structure

| Path | Purpose |
| :--- | :------ |
| `src/main.rs` | Entry point, server setup |
| `src/handlers.rs` | HTTP handlers |
| `src/views.rs` | HTML templates |
| `src/models.rs` | Data types |
| `src/database.rs` | SQLite operations |
| `src/utils.rs` | Utilities |

## Deployment

mdow deploys to Fly.io with LiteFS for distributed SQLite:

```bash
fly deploy
```

The `Dockerfile` builds the Rust binary. `litefs.yml` configures LiteFS. `fly.toml` defines the app and region.

## Contributing

Issues and PRs at [yree/mdow](https://github.com/yree/mdow).

## License

MIT.
