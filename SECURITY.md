# Security & Privacy Policy

## Privacy & Security Guarantees

`tomatui` is designed to be safe, local-first, and easy to audit:

- **100% Offline & Zero Telemetry**: `tomatui` makes zero network requests at runtime and contains no HTTP client, analytics, or telemetry dependencies.
- **Local-Only Data**: All projects, activities, sessions, and settings are stored exclusively in a local SQLite database on your machine. You can inspect its location (`tomatui db-path`), relocate it in Settings, export it (`tomatui export`), or delete it at any time.
- **Memory Safety**: `unsafe_code = "forbid"` is enforced at the crate level in `Cargo.toml`.
- **Agent Access**: AI agents interact with `tomatui` strictly through explicit local CLI commands (`--json`) or your local SQLite file — no open network ports, daemons, or remote servers.

## Supported Versions

| Version | Supported |
| :--- | :---: |
| `0.1.x` | Yes |

## Reporting a Vulnerability

If you discover a security vulnerability, please **do not** open a public issue. Instead, report it privately using **GitHub Security Advisories**:

1. Go to the [Security tab](https://github.com/kartik0trivedi/tomatui/security/advisories) of this repository.
2. Click **Report a vulnerability** and include reproduction steps and impact details.

You can expect an acknowledgment within 72 hours and a coordinated fix before public disclosure.
