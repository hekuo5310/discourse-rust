# Forum Engine

[中文](README.md)

[![Deploy to Cloudflare](https://deploy.workers.cloudflare.com/button)](https://deploy.workers.cloudflare.com/?url=https://github.com/hekuo5310/discourse-rust)

An independent, clean-room community platform written primarily in Rust.

The current tree does not contain or adapt source code, tests, assets, or prose
from the project that previously occupied this repository. Its old commits
remain in Git history. The temporary name “Forum Engine” is intentionally
neutral and may be replaced.

This project is licensed under the [Apache2.0](LICENSE).

## Implemented

- Rust domain crate shared by every runtime.
- Cloudflare Workers deployment compiled to WebAssembly.
- D1-backed users, sessions, categories, topics, and posts.
- KV session hints with D1 as the authoritative revocation source.
- Registration, login, logout, current-user, category, topic, and reply APIs.
- Responsive Workers Static Assets frontend for authentication, category browsing,
  topic creation, and replies.
- Native Rust HTTP runtime backed by PostgreSQL and Redis.
- Embedded PostgreSQL migrations and a container-based local stack.
- A WIT boundary for optional modules written in Rust, Go, C, C++, C#, Python,
  or any language that can target the WebAssembly Component Model.

Rust remains the primary implementation language. Other languages are accepted
only behind a versioned component boundary when they provide a concrete benefit.

## Deploy to Cloudflare Workers

Click the button above to copy this repository into your account. Cloudflare
will use `wrangler.jsonc` to provision D1 and KV automatically, apply the D1
migrations, build the Rust WebAssembly Worker, and configure Workers Builds for
automatic deployments from later commits.

Cloudflare's build image does not preinstall Rust. The shared build script
installs the minimal Rust toolchain on the first build and then generates the
Worker Wasm; the default Hello World code does not need to be kept manually.

The one-click path uses Workers + D1 + KV. The original PostgreSQL + Redis
option remains available through the native runtime below.

## Local Workers development

1. Install the JavaScript tooling:

   ```sh
   npm install
   ```

2. Apply the local D1 migrations:

   ```sh
   npm run migrate:local
   ```

3. Start the Worker:

   ```sh
   npm run dev
   ```

The API uses `Authorization: Bearer <token>`. The login and registration
responses return the token exactly once; only its SHA-256 digest is stored.

## Native quick start

The native runtime keeps the conventional PostgreSQL and Redis deployment
option. Start the complete stack with:

```sh
docker compose up --build
```

The API is then available at `http://localhost:3000`. To run the binary outside
the container, set `DATABASE_URL`, `REDIS_URL`, and optionally `LISTEN_ADDR`.
PostgreSQL is authoritative for sessions; Redis is an expendable cache hint.
Migrations run automatically during startup.

## API surface

| Method | Path | Authentication |
| --- | --- | --- |
| GET | `/api/v1/health` | no |
| POST | `/api/v1/auth/register` | no |
| POST | `/api/v1/auth/login` | no |
| POST | `/api/v1/auth/logout` | yes |
| GET | `/api/v1/me` | yes |
| GET | `/api/v1/categories` | no |
| POST | `/api/v1/categories` | administrator |
| GET | `/api/v1/topics` | no |
| POST | `/api/v1/topics` | yes |
| GET | `/api/v1/topics/:id` | no |
| POST | `/api/v1/topics/:id/posts` | yes |

The Workers/D1/KV and native/PostgreSQL/Redis runtimes expose this same contract.
See [`docs/ROADMAP.md`](docs/ROADMAP.md) for the remaining clean-room replacement
phases.

## Ownership and contributions

External code contributions are not accepted yet. This prevents accidental
mixed copyright ownership while the project establishes a contributor agreement
that permits future relicensing. Issues containing behavior descriptions are
welcome, but patches copied from another forum implementation are not.
