# Changelog: notes plugin 0.4.0

Written for people upgrading from 0.3; every entry is a user-visible change.

## Added

- A `notes.search` command that takes a **query string** and returns matches as JSON:
  ```json
  {"matches":[{"file":"a.md","line":12,"preview":"…"}]}
  ```
- `--follow` keeps the process alive and streams events on stdout, one object per line.
- Config keys may now be given in `notes.toml` instead of the environment.

## Changed

1. Events carry `seq` so a client can drop duplicates after a reconnect.
2. The default timeout is 30 s, up from 10 s.
   - Existing clients that rely on a fast failure should set `timeout = 2` explicitly.
   - The timeout applies per request, not per connection.

## Removed

- The undocumented `legacy_path` key. It was accepted but never read.
- Support for plugin protocol version 1; the host speaks 2 and up.

> If you still see `protocol mismatch`, run `notes --version` and check the host's
> `min_protocol` value. Anything below 2 refuses to start on purpose.

## Upgrade

```sh
gg notes install 0.4.0 --restart
gg notes doctor
```

The second command prints the negotiated protocol and the resolved config path.
