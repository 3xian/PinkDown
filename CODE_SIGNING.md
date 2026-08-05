# Code signing policy

For signed Windows releases, free code signing is provided by
[SignPath.io](https://signpath.io/), certificate by
[SignPath Foundation](https://signpath.org/).

Official release artifacts are built from this public repository by the
[release workflow](https://github.com/3xian/PinkDown/actions/workflows/release.yml).
Every signing request must originate from that workflow and must be manually
approved before publication.

## Team roles

- Committer and reviewer: [3xian](https://github.com/3xian)
- Signing approver: [3xian](https://github.com/3xian)

Changes from contributors who do not have commit access are reviewed before
they are merged. The maintainer uses multi-factor authentication for GitHub and
SignPath access.

## Privacy policy

This program will not transfer any information to other networked systems
unless specifically requested by the user or the person installing or
operating it.

When the user chooses **Check updates**, PinkDown contacts GitHub to list
PinkDown release tags. If an update is available and the user confirms
**Update**, PinkDown also downloads the platform release package and its
SHA-256 checksum from GitHub (Windows setup EXE or macOS DMG). If the user has
configured `GITHUB_TOKEN` or `GH_TOKEN`, that token is sent only to GitHub for
the requested API call. PinkDown has no telemetry, advertising, analytics, or
background network activity.
