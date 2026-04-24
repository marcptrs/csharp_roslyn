# C# Roslyn Extension for Zed

A Zed extension for C# with:

- **Roslyn language server** as the preferred backend
- **OmniSharp** as a compatibility fallback
- **netcoredbg** for debugging

## Current backend strategy

This extension supports both language servers:

- **Roslyn** (`roslyn-language-server`) is preferred when a **.NET 10+ runtime** is available.
- **OmniSharp** is selected automatically for Unity projects or when .NET 10+ is not detected.
- A configured custom Roslyn binary is always honored.

## Features

- Completion, hover, go to definition, find references, rename
- Diagnostics and analyzers
- Roslyn workspace configuration support
- Unity-oriented OmniSharp fallback
- Debugging via `netcoredbg`
- Automatic download of Roslyn/OmniSharp/netcoredbg when needed (platform-aware source selection)
- Support for custom language server binaries in Zed settings

## Semantic highlighting

Zed now supports semantic tokens. For C#, the main requirement is that the active language server advertises semantic token support and that Zed has semantic tokens enabled.

This extension also ships default semantic token mappings for Roslyn custom token types such as regex, JSON, XML doc, punctuation, verbatim strings, and extension methods.

Recommended setting:

```json
{
  "languages": {
    "CSharp": {
      "semantic_tokens": "full"
    }
  }
}
```

Use `"combined"` if you want tree-sitter highlighting with semantic overlays.

Notes:

- **Roslyn** is the preferred backend for semantic highlighting work.
- **OmniSharp** remains a compatibility path and may not match Roslyn's behavior.
- If highlighting does not update after changing this setting, run `lsp: restart language servers`.

## Installation

This extension is intended for local development installation.

1. Clone this repository
2. Open Zed
3. Run `zed: extensions: install dev extension`
4. Select the cloned `csharp_roslyn` directory
5. Restart Zed

## Configuration

### Select language server priority

You can configure language-server priority for C#:

```json
{
  "languages": {
    "CSharp": {
      "language_servers": ["roslyn", "!omnisharp", "..."]
    }
  }
}
```

To prefer OmniSharp explicitly:

```json
{
  "languages": {
    "CSharp": {
      "language_servers": ["omnisharp", "!roslyn", "..."]
    }
  }
}
```

Even when `roslyn` is preferred, this extension may still fall back to OmniSharp automatically when Roslyn is not usable on the host.

### Roslyn settings

Roslyn configuration goes under `lsp.roslyn.settings`:

```json
{
  "lsp": {
    "roslyn": {
      "settings": {
        "csharp|completion": {
          "dotnet_show_completion_items_from_unimported_namespaces": true
        },
        "csharp|navigation": {
          "dotnet_navigate_to_decompiled_sources": true,
          "dotnet_navigate_to_source_link_and_embedded_sources": true
        },
        "csharp|inlay_hints": {
          "dotnet_enable_inlay_hints_for_parameters": true,
          "csharp_enable_inlay_hints_for_types": true
        }
      }
    }
  }
}
```

### Custom Roslyn binary

If you installed `roslyn-language-server` yourself, you can point Zed to it:

```json
{
  "lsp": {
    "roslyn": {
      "binary": {
        "path": "/path/to/roslyn-language-server",
        "arguments": ["--stdio", "--autoLoadProjects"]
      }
    }
  }
}
```

This is useful if you want to manage the Roslyn server outside the extension.

### OmniSharp settings

OmniSharp configuration goes under `lsp.omnisharp`.

To set a solution path:

```json
{
  "lsp": {
    "omnisharp": {
      "initialization_options": {
        "solution": "/absolute/path/to/your/solution.sln"
      }
    }
  }
}
```

To use a custom OmniSharp binary:

```json
{
  "lsp": {
    "omnisharp": {
      "binary": {
        "path": "/path/to/OmniSharp",
        "arguments": ["-lsp"]
      }
    }
  }
}
```

### Unity projects

Unity currently routes through OmniSharp.

See [UNITY-SUPPORT.md](UNITY-SUPPORT.md) for details.

## Debugging

The extension provides `netcoredbg` support.

Notes:
- Linux, macOS x64, and Windows use official Samsung `netcoredbg` releases.
- macOS arm64 uses prebuilt arm64 assets from `marcptrs/netcoredbg` (upstream-style tag naming).

### Task-based debugging

Create `.zed/tasks.json`:

```json
[
  {
    "label": "Run MyApp",
    "command": "dotnet",
    "args": ["run", "--project", "src/MyApp/MyApp.csproj"],
    "use_new_terminal": false
  }
]
```

The extension can derive a debug scenario directly from `dotnet run` tasks.

### Manual debug configuration

Create `.zed/debug.json`:

```json
[
  {
    "label": "Debug MyApp",
    "adapter": "netcoredbg",
    "request": "launch",
    "program": "$ZED_WORKTREE_ROOT/src/MyApp/bin/Debug/$TARGET_FRAMEWORK/MyApp.dll",
    "args": [],
    "cwd": "$ZED_WORKTREE_ROOT",
    "stopAtEntry": false,
    "console": "internalConsole"
  }
]
```

`$TARGET_FRAMEWORK` is resolved from the corresponding `.csproj` when possible.

## Troubleshooting

### Enable debug logging

```json
{
  "lsp": {
    "roslyn": {
      "settings": {
        "enable_debug_logging": true
      }
    },
    "omnisharp": {
      "initialization_options": {
        "enable_debug_logging": true
      }
    }
  }
}
```

Logs appear in the terminal when running Zed in the foreground.

### Roslyn does not start

Common causes:

- No **.NET 10+ runtime** is installed.
- A Unity project was detected, so the extension intentionally fell back to OmniSharp.
- The custom Roslyn binary path is invalid.

In these cases, the extension should usually fall back to OmniSharp automatically.
When debug logging is enabled, the extension now logs the backend decision explicitly.

### Semantic highlighting not appearing

- Set `semantic_tokens` to `"full"` (or `"combined"`) for `CSharp`.
- Run `lsp: restart language servers` after changing the setting.
- Prefer Roslyn when comparing backend behavior.

### Debugger fails to launch

If you see startup errors (for example `configurationDone` failures), try:

- Ensure `program` points to an existing built DLL.
- Clear debugger cache so the adapter is re-downloaded:
  - `~/Library/Application Support/Zed/extensions/work/csharp_roslyn/cache/netcoredbg`
- Restart Zed and retry.

### No solution detected for OmniSharp

The extension first checks `lsp.omnisharp.initialization_options.solution`, then tries a simple heuristic for `<workspace-name>.sln`, `.slnx`, or `.slnf` in the workspace root.

If that still does not work, set it explicitly:

```json
{
  "lsp": {
    "omnisharp": {
      "initialization_options": {
        "solution": "/absolute/path/to/project.sln"
      }
    }
  }
}
```

## Development

Build:

```bash
cargo build --target wasm32-wasip2
```

Test:

```bash
cargo test
```

Install dev build:

```bash
cp target/wasm32-wasip2/debug/csharp_roslyn.wasm ~/Library/Application\ Support/Zed/extensions/installed/csharp_roslyn/extension.wasm
```

## Links

- Roslyn language server: https://raw.githubusercontent.com/dotnet/roslyn/refs/heads/main/src/LanguageServer/Microsoft.CodeAnalysis.LanguageServer/README.md
- Zed C# language docs: ../zed/docs/src/languages/csharp.md
- OmniSharp: https://github.com/OmniSharp/omnisharp-roslyn
- netcoredbg (official): https://github.com/Samsung/netcoredbg
- netcoredbg (macOS arm64 release pipeline): https://github.com/marcptrs/netcoredbg
