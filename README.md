# C# Roslyn Extension for Zed

A Zed editor extension providing C# language support via OmniSharp-Roslyn and netcoredbg debugger.

## Features

- Code completion, go-to-definition, find references, rename symbol
- Diagnostics and code analysis
- Solution file detection (.sln, .slnx, .slnf)
- MSBuild integration
- Debugging support via netcoredbg
- Auto-downloads OmniSharp-Roslyn and netcoredbg on first use
- Supports custom OmniSharp installations via PATH

### Known Limitations

- Go-to-definition on BCL (Base Class Library) types currently not working
- Some cross-project navigation may be limited

## Installation

This extension is currently in development and **not available in the Zed extension registry**.

To install:

1. Clone this repository
2. Open Zed and run: `zed: extensions: install dev extension`
3. Select the cloned `csharp_roslyn` directory
4. Restart Zed

## Setup

### Basic Usage

The extension works out of the box! Simply open a C# project with a `.sln` file and start coding.

### Optional: Specify Solution Path

If auto-detection doesn't find your solution file, specify it in your Zed settings (`Cmd+,` / `Ctrl+,`):

```json
{
  "language_servers": {
    "omnisharp-roslyn": {
      "initialization_options": {
        "solution": "/absolute/path/to/your/solution.sln"
      }
    }
  }
}
```

### Optional: Use Custom OmniSharp Installation

```json
{
  "language_servers": {
    "omnisharp-roslyn": {
      "enabled": true,
      "initialization_options": {
        "solution": "/absolute/path/to/your/solution.sln"
      }
    }
  }
}
```

## Debugging

The extension supports automatic debug configuration generation from tasks or manual configuration.

### Option 1: Tasks with Auto-Generated Debug Config (Recommended)

Create `.zed/tasks.json` in your project root:

```json
[
  {
    "label": "Run MyApp",
    "command": "dotnet",
    "args": ["run", "--project", "src/MyApp/MyApp.csproj"],
    "use_new_terminal": false
  },
  {
    "label": "Build MyApp",
    "command": "dotnet",
    "args": ["build", "src/MyApp/MyApp.csproj"],
    "use_new_terminal": false
  }
]
```

The extension automatically generates debug configurations from your `dotnet run` tasks. Simply select the task from the debug panel and start debugging!

### Option 2: Manual Debug Configuration

Create `.zed/debug.json` in your project root:

```json
[
  {
    "label": "Debug MyApp",
    "adapter": "netcoredbg",
    "request": "launch",
    "program": "$ZED_WORKTREE_ROOT/src/MyApp/bin/Debug/{targetFramework}/MyApp.dll",
    "args": [],
    "cwd": "$ZED_WORKTREE_ROOT",
    "stopAtEntry": false,
    "console": "internalConsole"
  }
]
```

**Note**: The `{targetFramework}` placeholder (e.g., `net8.0`, `net9.0`) is automatically detected from your `.csproj` file at debug time.

### Debug Configuration Options

- `program`: Path to the .NET DLL to debug (use `$ZED_WORKTREE_ROOT` for workspace root)
- `args`: Array of command-line arguments
- `cwd`: Working directory
- `env`: Environment variables object
- `stopAtEntry`: Break at program entry point (default: false)
- `console`: `"internalConsole"`, `"integratedTerminal"`, or `"externalTerminal"`

### Example: Debug with Arguments

```json
{
  "label": "Debug with Args",
  "adapter": "netcoredbg",
  "request": "launch",
  "program": "$ZED_WORKTREE_ROOT/bin/Debug/{targetFramework}/MyApp.dll",
  "args": ["--verbose", "input.txt"],
  "env": {
    "ASPNETCORE_ENVIRONMENT": "Development"
  },
  "cwd": "$ZED_WORKTREE_ROOT"
}
```

## Troubleshooting

### Language Server Won't Start

- Extension auto-downloads OmniSharp-Roslyn on first use
- Cache location: `~/.cache/zed/extensions/csharp_roslyn/cache/` (Linux/macOS) or `%LOCALAPPDATA%\Zed\extensions\csharp_roslyn\cache\` (Windows)
- Check logs: `Help → View Logs` in Zed
- Manually download from: https://github.com/OmniSharp/omnisharp-roslyn/releases

### Debugger Issues

- Extension auto-downloads netcoredbg on first use
- Verify `program` path exists and build completed successfully
- Ensure Debug configuration (not Release)
- Check logs: `Help → View Logs`

### No Solution File Detected

- Place `.sln` file in workspace root, or
- Manually specify solution path in settings (see Setup section above)

## Development

### Building

```bash
cargo build --target wasm32-wasip2 --release
```

### Installing Dev Build

```bash
cp target/wasm32-wasip2/release/csharp_roslyn.wasm ~/Library/Application\ Support/Zed/extensions/installed/csharp_roslyn/extension.wasm
```

## License

MIT License

## Links

- **OmniSharp-Roslyn**: https://github.com/OmniSharp/omnisharp-roslyn
- **netcoredbg**: https://github.com/Samsung/netcoredbg
- **Issues**: https://github.com/marcptrs/csharp-roslyn/issues
