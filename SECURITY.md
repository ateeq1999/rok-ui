# Security Policy

## Supported versions

rok-ui is pre-1.0. Security fixes go into the latest release only.

| Version | Supported |
|---|---|
| 0.2.x | Yes |
| < 0.2 | No |

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

rok-ui is a UI library: it does not open network connections or run untrusted code itself.
Issues most likely to matter are crashes or memory problems triggered by input the
application passes in, such as text, images or file names.
