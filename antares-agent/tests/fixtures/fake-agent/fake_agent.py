#!/usr/bin/env python3
"""Deterministic mock ACP child agent for Antares headless integration tests.

Supported scenarios:
  --scenario echo         (default): Normal ACP session handshake, prompt streaming, shutdown
  --scenario stderr-flood : Writes thousands of lines to stderr while responding on stdout
  --scenario crash        : Exits immediately with exit code 42
  --scenario delay        : Sleeps before replying to simulate timeouts
  --scenario malformed    : Emits invalid JSON-RPC frames
"""
import argparse
import json
import sys
import time

def handle_echo():
    for raw in sys.stdin:
        line = raw.strip()
        if not line:
            continue
        try:
            msg = json.loads(line)
        except Exception:
            continue

        req_id = msg.get("id")
        method = msg.get("method")

        if method == "initialize":
            resp = {
                "jsonrpc": "2.0",
                "id": req_id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "agentInfo": {"name": "mock-octomind", "version": "0.72.0"},
                    "capabilities": {"streaming": True}
                }
            }
            sys.stdout.write(json.dumps(resp) + "\n")
            sys.stdout.flush()

        elif method == "session/new":
            resp = {
                "jsonrpc": "2.0",
                "id": req_id,
                "result": {
                    "sessionId": "mock-session-001",
                    "state": "ready"
                }
            }
            sys.stdout.write(json.dumps(resp) + "\n")
            sys.stdout.flush()

        elif method == "session/prompt":
            # Stream 2 token notifications
            for chunk in ["Hello ", "from mock agent!"]:
                notif = {
                    "jsonrpc": "2.0",
                    "method": "session/update",
                    "params": {
                        "sessionId": "mock-session-001",
                        "update": {"textDelta": chunk}
                    }
                }
                sys.stdout.write(json.dumps(notif) + "\n")
                sys.stdout.flush()

            # Final prompt response
            resp = {
                "jsonrpc": "2.0",
                "id": req_id,
                "result": {"status": "completed"}
            }
            sys.stdout.write(json.dumps(resp) + "\n")
            sys.stdout.flush()

        elif method == "session/cancel":
            sys.stderr.write("received cancel notification\n")
            sys.stderr.flush()

def handle_stderr_flood():
    for i in range(1000):
        sys.stderr.write(f"diagnostic log line {i}\n")
    sys.stderr.flush()
    handle_echo()

def handle_delay():
    for raw in sys.stdin:
        time.sleep(2.0)
        line = raw.strip()
        if not line:
            continue
        msg = json.loads(line)
        resp = {"jsonrpc": "2.0", "id": msg.get("id"), "result": {}}
        sys.stdout.write(json.dumps(resp) + "\n")
        sys.stdout.flush()

def handle_malformed():
    for _ in sys.stdin:
        sys.stdout.write("MALFORMED JSON FRAME\n")
        sys.stdout.flush()

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--scenario", default="echo", choices=["echo", "stderr-flood", "crash", "delay", "malformed"])
    args = parser.parse_args()

    if args.scenario == "crash":
        sys.stderr.write("crashing with exit code 42\n")
        sys.exit(42)
    elif args.scenario == "stderr-flood":
        handle_stderr_flood()
    elif args.scenario == "delay":
        handle_delay()
    elif args.scenario == "malformed":
        handle_malformed()
    else:
        handle_echo()

if __name__ == "__main__":
    main()
