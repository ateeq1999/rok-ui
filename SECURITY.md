# Security Policy

## Supported versions

rok-ui is pre-1.0. Security fixes go into the latest release only.

| Version | Supported |
|---|---|
| 0.8.x | Yes |
| < 0.8 | No |

## Reporting a vulnerability

Please do not open a public issue for a security problem.

Report it privately through GitHub instead: on the repository page, open **Security** →
**Report a vulnerability**. Include:

- the affected version,
- what an attacker can do and under which conditions,
- steps or a snippet that reproduces it.

You should get a reply within a week. Once the problem is confirmed, a fix is released as soon
as practical and the advisory is published with credit to you, unless you prefer to stay
anonymous.

rok-ui is a UI library: it does not run untrusted code itself, and it opens network
connections only through the opt-in `db` feature (PostgreSQL through rok-db, to the URL the
application passes in). Issues most likely to matter are crashes or memory problems triggered
by input the application passes in, such as text, images or file names. Report problems in
rok-db's query building (for example SQL injection) to
[rok-db](https://github.com/ateeq1999/rok-db/security) instead.

Dependencies are checked for known vulnerabilities and license problems in CI with
`cargo deny` (see `deny.toml`).
