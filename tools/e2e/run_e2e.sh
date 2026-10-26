#!/usr/bin/env bash
# End-to-end check without hardware: intiface_engine from this fork + fake device + spec v3 client.
# Usage: tools/e2e/run_e2e.sh [path-to-intiface_engine]   (default: builds target/release/intiface_engine)
set -u
cd "$(dirname "$0")/../.."
ENGINE="${1:-target/release/intiface_engine}"
if [ ! -x "$ENGINE" ]; then
  cargo build --release -p intiface-engine || exit 1
fi
OUT="${E2E_OUT:-/tmp/hismith-e2e}"
mkdir -p "$OUT"
status=0
for kind in s2 piupiu; do
  flag=""; [ "$kind" = piupiu ] && flag="--piupiu"
  "$ENGINE" --websocket-port 12345 --use-device-websocket-server --device-websocket-server-port 54817 \
    --user-device-config-file tools/e2e/user-config-websocket.json --log info > "$OUT/engine-$kind.log" 2>&1 &
  engine_pid=$!
  sleep 2
  python3 tools/e2e/fake_hismith.py $flag --out "$OUT/frames-$kind.txt" 2> "$OUT/fake-$kind.log" &
  fake_pid=$!
  sleep 1
  python3 tools/e2e/v3_client_check.py $flag --frames "$OUT/frames-$kind.txt" --device-added "$OUT/device-added-$kind.json" | tee "$OUT/result-$kind.txt"
  rc=${PIPESTATUS[0]}
  [ "$rc" -ne 0 ] && status=1
  kill $fake_pid $engine_pid 2>/dev/null; wait $fake_pid $engine_pid 2>/dev/null
done
echo "frames and logs in $OUT"
exit $status
