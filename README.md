# todo-list

A single list of the things you have to do by hand, gathered from AI agents and other apps.
Open it on a phone, tap an item's link to go and act, then check it off.

Each to-do is one Markdown file. The browser UI, an HTTP API and an MCP endpoint at `/mcp`
read and write the same files. This repository holds only the application code.
Your to-dos live in your data directory and in any backup archives you create;
keep the R2 bucket you upload them to private.

The service has no built-in authentication: anyone who can reach its port can read and change
the list. Run it on a home LAN or tailnet and control access there (Tailscale, a firewall or a proxy).

## Run with Docker

```sh
docker run -d --name todo-list \
  -p 127.0.0.1:3000:3000 \
  -v todo-list-data:/app/data \
  ghcr.io/miyabisun/todo-list:latest
```

Open <http://127.0.0.1:3000>. To-dos are stored in the `todo-list-data` volume at
`/app/data/todos`; keep the volume when you replace the container.
The container runs as UID/GID `10001:10001`. A host bind mount must be writable by that user.
Change `127.0.0.1` in `-p` to the address your LAN or tailnet reaches.

Add a first item from the command line (the UI has the same form):

```sh
curl --fail-with-body http://127.0.0.1:3000/api/todos \
  -H 'Content-Type: application/json' \
  -d '{"title":"Return the library book","url":"https://example.com/loans","due":"2026-10-01","source":"readme"}'
```

## Configuration

All settings are optional and read at startup.

| Variable | Default | Meaning |
| --- | --- | --- |
| `APP_DATA_DIR` | `data/todos` (image: `/app/data/todos`) | Directory of to-do files, relative to the working directory. Created when missing. A blank value fails startup. |
| `PORT` | `3000` | TCP port from 1 to 65535. Invalid values fail startup. The server listens on `0.0.0.0`. |
| `LOG_LEVEL` | `info` | `off`, `error`, `warn`, `info`, `debug` or `trace`. Anything else means `info`. |

## To-do files

`APP_DATA_DIR/<id>.md` holds one to-do. The server assigns a UUID as the ID.
Hand-written files work too when their name is 1–128 letters, digits, `-` or `_`, plus `.md`.

```markdown
---
title: Return the library book
url: https://example.com/loans
due: 2026-10-01
source: readme
created_at: 2026-09-26T07:00:00.000Z
updated_at: 2026-09-26T07:00:00.000Z
done_at: null
---
Optional memo in Markdown.
```

| Field | Meaning |
| --- | --- |
| `title` | Required. One non-blank line. |
| `url` | Optional `http://` or `https://` link the user taps to act. Other schemes are rejected. |
| `due` | Optional due date, `YYYY-MM-DD`. |
| `source` | Optional name of the app or agent that registered it. |
| `done_at` | `null` while open; the completion time once done. Completing again keeps the first time. |
| body | The memo after the closing `---`. The UI shows it as plain text. |

An item is either open or done. Delete an item you no longer need through the API.
Other frontmatter fields you add by hand survive edits made through the server;
`id` and `body` are reserved (the file name is the ID and the memo follows the fence).
Each write replaces one file atomically. Run only one server per directory.

## HTTP API

| Request | Result |
| --- | --- |
| `GET /api/todos` | All to-dos: open ones by due date (undated last, then oldest first), then done ones (newest first). `?done=false` or `?done=true` filters. |
| `POST /api/todos` | Create from `{title, url?, due?, source?, body?}`. Returns 201 with the saved item. |
| `GET /api/todos/{id}` | One to-do, including `body`. |
| `PATCH /api/todos/{id}` | Change any of `title`, `url`, `due`, `source`, `body`, `done`. `null` clears `url`, `due` or `source`; `done: true` completes, `done: false` reopens. |
| `DELETE /api/todos/{id}` | Remove the file. Returns 204. |
| `GET /api/snapshot` | `{"files": {"<id>.md": "<file text>"}}`, read under the write lock for backups. |
| `GET /healthz`, `GET /api/health` | `ok` and `{"status":"ok"}`. |

Requests with a body use `Content-Type: application/json`. Errors return
`{"error": "...", "code": "invalid" | "not_found" | "io"}` with status 400, 404 or 500.
Unknown or read-only fields (`id`, timestamps) are rejected, and nothing is written.

## MCP

The Streamable HTTP endpoint is `http://<host>:<port>/mcp`. For example, with Claude Code:

```sh
claude mcp add --transport http todo-list http://127.0.0.1:3000/mcp
```

| Tool | Arguments | Result |
| --- | --- | --- |
| `todo_list` | `done?` (default `false`), `limit?` (1–200, default 50), `offset?` | `{todos, total, next_offset}`; entries omit the memo. |
| `todo_get` | `id` | The full to-do. |
| `todo_add` | `title`, `url?`, `due?`, `source?`, `body?` | The saved to-do. |
| `todo_update` | `id` plus fields as in `PATCH` | The saved to-do. |
| `todo_complete` | `id` | The saved to-do with `done_at`. |

Invalid arguments come back as a tool error with the same `error` and `code` as the HTTP API.
The server's MCP instructions ask agents to set `source` to their own name and to give a
tappable `url` when there is one.

## Backup and restore

`bin/todo-data` (Python 3 standard library) saves the server's snapshot as a compressed archive
with a SHA-256 manifest. Run it where it can reach the server:

```sh
bin/todo-data snapshot --server http://127.0.0.1:3000 --output-dir /backups/todo-list
bin/todo-data restore /backups/todo-list/todos-TIMESTAMP.tar.gz /srv/todo-list/restored
```

Each snapshot is a new `todos-<UTC timestamp>.tar.gz`; an existing archive is never overwritten.
`restore` checks every entry name and checksum before it creates the destination,
and refuses a destination that already exists. Point `APP_DATA_DIR` at the restored
directory (or copy its files into a stopped server's directory) to use it.

To upload archives to Cloudflare R2, install the AWS CLI (`aws`) and set these variables
outside the repository, then use `snapshot --upload` or `upload <archive>`:

| Variable | Meaning |
| --- | --- |
| `R2_ENDPOINT` | `https://<account id>.r2.cloudflarestorage.com` |
| `R2_BUCKET` | Destination bucket. |
| `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY` | R2 API token with write access to the bucket. |
| `R2_PREFIX` | Optional key prefix, default `todo-list`. |

The upload fails before calling `aws` and names every missing variable.
The local archive is kept either way. Uploads never delete older generations;
set retention on the bucket. To restore from R2, download an archive
(for example `aws s3 cp s3://<bucket>/todo-list/<archive> .` with the same endpoint and
credentials) and run `restore`.

## Development

Requires Rust 1.96+, Node.js 24 with npm, and Python 3. Build the frontend before Rust,
because the binary embeds `client/dist`:

```sh
npm --prefix client ci
npm --prefix client run build
cargo run --locked
```

For frontend work, run `npm --prefix client run dev` in a second terminal and open
<http://127.0.0.1:5173>; Vite proxies `/api` to port 3000.
[`DESIGN.md`](DESIGN.md) is the UI contract.

Checks to pass locally before every commit (CI does not run them):

```sh
npm --prefix client run format:check
npm --prefix client run check
npm --prefix client test
npm --prefix client run build
npm --prefix client run lint:design
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --locked
python3 -m unittest discover -s tests -p "test_*.py"
cargo build --locked
npx --prefix client playwright install chromium
npm --prefix client run test:e2e
docker build -t todo-list:test .
bash tests/smoke-image.sh todo-list:test
```

The smoke test needs `curl` and `jq` and exercises the image with an empty volume.
The browser tests start `target/debug/todo-list` on port 5187 with a temporary data directory,
seed fictional items and remove the directory afterwards.
Set `E2E_EVIDENCE_DIR` to keep screenshots and layout measurements.

A `vMAJOR.MINOR.PATCH` tag that matches `Cargo.toml` starts the release workflow:

1. It builds the image and publishes `ghcr.io/miyabisun/todo-list:<version>` and `:latest`.
2. A GitHub Release records the image digest.

## License

[MIT](LICENSE)
