# Clean-room reconstruction roadmap

This is a phased independent implementation, not a source translation. A phase
is complete only after its behavior is covered by automated tests in every
supported runtime.

## Completed

1. Repository replacement and provenance policy.
2. Rust domain core and versioned HTTP contract.
3. Cloudflare Workers runtime with D1 and KV.
4. Native Rust runtime with PostgreSQL and Redis.
5. Accounts, sessions, categories, topics, and replies.

## Next

1. Authorization policy, moderation roles, and audit events.
2. Topic editing, deletion, closing, pinning, and category permissions.
3. User profiles, preferences, notifications, and trust progression.
4. Search, uploads, email delivery, and background jobs.
5. Server-rendered web interface, accessibility, localization, and themes.
6. Import/export tooling, operational telemetry, backups, and upgrade testing.
7. Extension component host and stable WIT compatibility suite.

The project will not claim feature parity until the remaining phases have API,
storage, security, and end-to-end coverage for both deployment modes.
