# Security policy

## Supported versions

Pebble is pre-1.0 and evolving. Security fixes are applied to `main`.

## Reporting a vulnerability

Please do not open a public issue for a security problem. Instead, use GitHub's
private reporting:

1. Go to the repository's **Security** tab.
2. Choose **Report a vulnerability** (GitHub Security Advisories).

If that is unavailable, contact the maintainer directly at the address on their
GitHub profile. You can expect an initial response within a few days.

## Scope

Pebble executes the programs you give it: it has no sandbox and no resource
limits beyond the recursion and frame caps. Running untrusted Pebble source is
therefore equivalent to running untrusted code, and is not considered a
vulnerability by itself. The same goes for the C shim in `pebble-sys`, which
trusts the caller's buffer lengths.
