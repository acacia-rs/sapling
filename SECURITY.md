# Security policy

## Supported versions

sapling is pre-release. Only the latest commit on `master` gets fixes.

## Reporting a vulnerability

Please don't open a public issue. Report it privately through GitHub's
[**Report a vulnerability**](https://github.com/acacia-rs/sapling/security/advisories/new) form, with
the commit you tested, what a client has to send, and what happens to the server.

You'll get a reply within a week. Once a fix is on `master`, the advisory is published with credit to
you unless you'd rather stay anonymous.

## Scope

sapling accepts connections from anyone who can reach it, so these count:

- anything a client can send that crashes a server, stalls a shard or makes memory grow without bound
- logging in as someone else, or getting past online-mode (Xbox Live) verification
- getting around what a plugin cancelled: protection, permissions, command checks
- reading or changing another player's data or a world the player was not sent to

Bugs in vanilla gameplay that only give an advantage in game (duplication, reach) are ordinary issues.
