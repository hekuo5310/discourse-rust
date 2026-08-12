# Clean-room development policy

The project is an independent implementation of a community discussion system.
Its code is written from product requirements, public protocol behavior, web
standards, and Cloudflare/Rust documentation.

## Prohibited inputs

- Source code, tests, migrations, assets, translations, documentation prose, or
  database queries copied or translated from another forum implementation.
- Mechanical language-to-language ports.
- Screenshots used to reproduce distinctive visual expression pixel-for-pixel.
- Registered names, logos, icons, mascots, or other third-party branding.
- Patches from contributors who have not agreed to the project's relicensing
  terms once a contributor agreement is introduced.

## Permitted inputs

- Independently written product requirements.
- Public HTTP and data-format specifications.
- Generic forum concepts such as users, categories, topics, and replies.
- Observed interoperability behavior recorded as facts, without copying the
  original implementation's test text or internal structure.
- Documentation for dependencies and deployment platforms.

## Review record

Every pull request must state its behavioral sources and certify that it contains
no copied or translated implementation material. The repository owner retains
the right to reject code whose provenance is unclear.

