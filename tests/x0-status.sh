#!/bin/sh
set -eu

dbus-run-session -- sh -eu -c '
    target/debug/xbotd &
    daemon=$!
    trap '\''kill "$daemon" 2>/dev/null || :; wait "$daemon" 2>/dev/null || :'\'' EXIT
    count=0
    while [ "$count" -lt 30 ]; do
        if target/debug/crux xbot status --json > "$1" 2>/dev/null; then
            break
        fi
        count=$((count + 1))
        sleep 0.1
    done
    test "$count" -lt 30
    target/debug/crux help --json > "$2"
' sh "$1" "$2"
