#!/usr/bin/env python3
"""
Quick semantic tokens probe against Roslyn via stdio.
Requirements: Roslyn LSP server binary available (downloaded by wrapper) or provide path via ROSLYN_LSP_PATH.
Usage: python test_semantic_tokens.py /absolute/path/to/file.cs
Prints decoded semantic tokens with line/char spans and kinds/modifiers.
"""

import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

ROSLYN_ENV_VAR = "ROSLYN_LSP_PATH"
DEFAULT_CACHE = (
    Path.home() / ".cache" / "roslyn-wrapper" / "Microsoft.CodeAnalysis.LanguageServer"
)

_id_counter = 0


def next_id():
    global _id_counter
    _id_counter += 1
    return _id_counter


def find_roslyn_binary():
    # Roslyn LSP is a dotnet DLL entry; we run via 'dotnet <path>'. Accept env override.
    env_path = os.environ.get(ROSLYN_ENV_VAR)
    if env_path and Path(env_path).exists():
        return Path(env_path)
    if DEFAULT_CACHE.exists():
        # Heuristic: pick first DLL matching pattern
        for dll in DEFAULT_CACHE.rglob("Microsoft.CodeAnalysis.LanguageServer.dll"):
            return dll
    raise SystemExit(
        "Could not locate Roslyn Language Server. Set ROSLYN_LSP_PATH or run wrapper first."
    )


def launch_roslyn(roslyn_path):
    dotnet = shutil.which("dotnet")
    if not dotnet:
        raise SystemExit("dotnet CLI not found on PATH.")
    proc = subprocess.Popen(
        [dotnet, str(roslyn_path)],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=1,
    )
    return proc


def send(proc, msg):
    data = json.dumps(msg)
    header = f"Content-Length: {len(data)}\r\n\r\n"
    proc.stdin.write(header + data)
    proc.stdin.flush()


def read_message(proc):
    # Very small naive LSP reader sufficient for this test
    headers = {}
    while True:
        line = proc.stdout.readline()
        if not line:
            return None
        line = line.rstrip("\r\n")
        if line == "":
            break
        if ":" not in line:
            continue
        k, v = line.split(":", 1)
        headers[k.strip().lower()] = v.strip()
    length = int(headers.get("content-length", 0))
    body = proc.stdout.read(length)
    return json.loads(body) if body else None


def main():
    if len(sys.argv) != 2:
        print("Usage: test_semantic_tokens.py /path/to/file.cs")
        sys.exit(1)
    file_path = Path(sys.argv[1]).resolve()
    if not file_path.exists():
        sys.exit(f"File not found: {file_path}")
    source = file_path.read_text()

    roslyn = find_roslyn_binary()
    proc = launch_roslyn(roslyn)

    initialize = {
        "jsonrpc": "2.0",
        "id": next_id(),
        "method": "initialize",
        "params": {
            "processId": None,
            "rootUri": str(file_path.parent.as_uri()),
            "capabilities": {
                "textDocument": {"semanticTokens": {"dynamicRegistration": False}},
                "workspace": {},
            },
            "workspaceFolders": [
                {"uri": str(file_path.parent.as_uri()), "name": file_path.parent.name}
            ],
        },
    }
    send(proc, initialize)
    init_resp = read_message(proc)
    if not init_resp:
        sys.exit("No initialize response.")
    send(proc, {"jsonrpc": "2.0", "method": "initialized", "params": {}})

    # Open document
    send(
        proc,
        {
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": file_path.as_uri(),
                    "languageId": "csharp",
                    "version": 1,
                    "text": source,
                }
            },
        },
    )

    # Request full semantic tokens
    sem_id = next_id()
    send(
        proc,
        {
            "jsonrpc": "2.0",
            "id": sem_id,
            "method": "textDocument/semanticTokens/full",
            "params": {"textDocument": {"uri": file_path.as_uri()}},
        },
    )

    legend = (
        init_resp.get("result", {})
        .get("capabilities", {})
        .get("semanticTokensProvider", {})
        .get("legend", {})
    )
    token_types = legend.get("tokenTypes", [])
    token_mods = legend.get("tokenModifiers", [])

    # Collect messages until semantic tokens response arrives
    sem_resp = None
    while True:
        msg = read_message(proc)
        if msg is None:
            break
        if msg.get("id") == sem_id:
            sem_resp = msg
            break
    if not sem_resp:
        sys.exit("No semantic tokens response received.")

    data = sem_resp.get("result", {}).get("data", [])
    if not data:
        print("Empty semantic tokens data.")
        sys.exit(0)

    # Decode per LSP spec (delta encoding: line, startChar, length, tokenType, tokenModifiersBitset)
    decoded = []
    line = 0
    col = 0
    for i in range(0, len(data), 5):
        d_line, d_col, length, ttype_i, tmods_bits = data[i : i + 5]
        line += d_line
        col = col + d_col if d_line == 0 else d_col
        ttype = token_types[ttype_i] if ttype_i < len(token_types) else f"<{ttype_i}>"
        mods = [m for bit, m in enumerate(token_mods) if tmods_bits & (1 << bit)]
        decoded.append(
            {
                "line": line,
                "start": col,
                "length": length,
                "type": ttype,
                "modifiers": mods,
            }
        )
    print(f"Semantic tokens ({len(decoded)}):")
    for tok in decoded[:200]:
        print(
            f"{tok['line']:>4}:{tok['start']:>3} len={tok['length']:>2} type={tok['type']} mods={','.join(tok['modifiers'])}"
        )

    proc.terminate()


if __name__ == "__main__":
    main()
