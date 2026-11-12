# Contributing and publishing safely

Use a reviewed work branch. Configure this checkout with a GitHub no-reply
author/committer identity and run `./scripts/check-public-metadata.ps1` before
every push. Never publish private assets, personal paths, credentials or account
contact details. Keep necessary third-party license notices intact.

`main` requires successful GitHub Actions checks named `verify (1.95.0)` and
`verify (1.99.0)`, including the full reachable public commit-history audit.
The requirement also applies to administrators. An actual unchecked update was
rejected by the server without changing main's source or head. This prevents an
unchecked direct documentation update from bypassing the existing audit.

Push `work/**` branches to run checks on their exact commits; PR events also
check integration. After review and all required checks pass, publish the exact
tested commit by fast-forwarding main. Recheck the remote base before updating
it. If main advanced, integrate that change on the work branch and verify again.
Do not synthesize an untested merge commit or use an API that silently inserts
the account's personal author/committer email. Do not disable protection or
force-push to get around a failed check.

The GitHub email-pattern ruleset was unavailable in this repository; protected
status checks are the enforced publication gate, not a general server-side email
filter. Other public branches still require correct no-reply identities before
their first push. Historical GitHub caches/internal refs may retain old commits;
no cleanup is a claim that those remote views were purged.

Release artifacts remain gated on exact-package Windows install, actual hover,
update, restart and rollback evidence, plus honest support/fixture status.
