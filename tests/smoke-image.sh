#!/usr/bin/env bash
# Exercise a built image with an empty volume: health, UI, API round trip, MCP, runtime user.
set -euo pipefail
image=${1:?usage: tests/smoke-image.sh IMAGE}
container=$(docker run --detach --publish 127.0.0.1::3000 "$image")
trap 'docker rm --force --volumes "$container" >/dev/null' EXIT
base="http://$(docker port "$container" 3000/tcp)"

for attempt in {1..40}; do
  if curl --fail --silent "$base/healthz" >/dev/null; then
    break
  fi
  if [[ "$attempt" == 40 ]]; then
    docker logs "$container"
    exit 1
  fi
  sleep 0.5
done

test "$(curl --fail --silent "$base/api/health")" = '{"status":"ok"}'
curl --fail --silent "$base/" | grep --quiet --ignore-case '<!doctype html'
test "$(curl --fail --silent "$base/api/todos")" = '[]'
id=$(curl --fail --silent -H 'content-type: application/json' \
  -d '{"title":"smoke item","url":"https://example.com/"}' "$base/api/todos" | jq -er .id)
curl --fail --silent -X PATCH -H 'content-type: application/json' -d '{"done":true}' \
  "$base/api/todos/$id" | jq -e '.done_at != null' >/dev/null
curl --fail --silent "$base/api/snapshot" | jq -e --arg f "$id.md" '.files | has($f)' >/dev/null
curl --fail --silent -H 'content-type: application/json' -H 'accept: application/json, text/event-stream' \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"smoke","version":"1"}}}' \
  "$base/mcp" | grep --quiet '"tools"'
docker exec "$container" sh -c "test \"\$(id -u)\" != 0 && test -f /app/data/todos/$id.md"
echo "smoke ok: $image"
