# C# Roslyn Extension for Zed

A Zed editor extension providing C# language support via Microsoft's Roslyn Language Server and netcoredbg debugger.

## Features

- **Language Server Protocol** integration with Roslyn for:
  - Code completion
  - Go-to-definition
  - Find references
  - Rename symbol
  - Diagnostics and code analysis
- **Debugging** support via netcoredbg
- **Solution file detection** - automatically detects `.sln`, `.slnx`, and `.slnf` files

## Prerequisites

### Roslyn Wrapper (Auto-Downloads Roslyn with Automatic Fallback)

This extension uses an **LSP proxy wrapper** (`roslyn-wrapper`) that:
- Proxies messages between Zed and the Roslyn Language Server
- **Automatically downloads Roslyn** from NuGet on first use (no manual installation needed!)
- **Tries multiple versions automatically** - if one version isn't available, tries the next
- **Falls back to globally installed Roslyn** if downloads fail
- Automatically injects the `solution/open` notification after initialization
- Enables advanced cross-project features

You only need to install the wrapper binary - Roslyn will be downloaded or located automatically.

#### Step 1: Build and Install the Wrapper

The wrapper binary must be built and available in your PATH.

**Option A: Build from Source** (if you have Rust installed)

```bash
cd roslyn_wrapper
cargo build --release
# Binary will be at: target/release/roslyn-wrapper (or .exe on Windows)
```

Then add it to your PATH or copy to a PATH directory:

```bash
# Linux/macOS
cp target/release/roslyn-wrapper /usr/local/bin/

# Windows
copy target\release\roslyn-wrapper.exe C:\Windows\System32\  # or add folder to PATH
```

**Option B: Download Pre-built Binary**

[Pre-built binaries will be available in releases - currently build from source]

#### Step 2: Auto-Download on First Use

When you open a C# file in Zed:
1. The wrapper will check the cache for any available version of Roslyn
2. If not cached, it attempts to download from NuGet (tries versions 5.0.0-1.25277.114, 4.12.0, 4.11.0, 4.10.0)
3. If downloads fail, looks for globally installed Roslyn via `dotnet tool install`
4. Extract it to the cache directory
5. Start the Roslyn Language Server

**Cache Location:**
- Linux/macOS: `~/.cache/roslyn-wrapper/{version}` or `$XDG_CACHE_HOME/roslyn-wrapper/{version}`
- Windows: `%LOCALAPPDATA%\roslyn-wrapper\cache\{version}` 

For example on Windows:
```
C:\Users\YourName\AppData\Local\roslyn-wrapper\cache\5.0.0-1.25277.114\
C:\Users\YourName\AppData\Local\roslyn-wrapper\cache\4.12.0\
```

You'll see messages like:
```
[roslyn-wrapper] Trying to download Roslyn 5.0.0-1.25277.114 from NuGet...
[roslyn-wrapper] Downloaded XXX bytes
[roslyn-wrapper] Successfully installed Roslyn 5.0.0-1.25277.114 at: [cache path]
```

#### Step 3: Verify Installation

Ensure the wrapper is in your PATH:

```bash
# Linux/macOS
which roslyn-wrapper

# Windows (PowerShell)
Get-Command roslyn-wrapper

# Windows (Command Prompt)
where roslyn-wrapper
```

If not found, ensure it's installed and added to your PATH.

### Debugger (Auto-Downloads)

The extension includes **automatic netcoredbg installation**. When you start a debug session:
1. The extension checks if netcoredbg is already cached
2. If not found, it downloads the appropriate version for your platform from GitHub
3. Extracts it to the extension cache directory
4. Starts the debugger automatically

**No manual installation required!** The debugger will be downloaded on first use.

**Cache Location:**
- Linux/macOS: `~/.cache/zed/extensions/csharp_roslyn/cache/netcoredbg/{version}/`
- Windows: `%LOCALAPPDATA%\Zed\extensions\csharp_roslyn\cache\netcoredbg\{version}\`

**Optional**: If you prefer to use a globally installed netcoredbg:
```bash
# macOS with Homebrew
brew install netcoredbg

# Linux (download from GitHub)
# https://github.com/Samsung/netcoredbg/releases

# Windows (download from GitHub)
# https://github.com/Samsung/netcoredbg/releases
```

## Setup & Usage

1. **Install the Extension** in Zed:
   ```
   zed: install extension
   ```
   Search for `csharp_roslyn` and install

2. **Create/Open a C# Project**:
   - Open a folder containing a `.sln`, `.slnx`, or `.slnf` file
   - The extension automatically detects the solution file

3. **Enable the Language Server**:
   - In Zed settings (`Cmd+,` / `Ctrl+,`), enable the Roslyn language server:
   ```json
   {
     "language_servers": {
       "roslyn": {
         "enabled": true
       }
     }
   }
   ```

4. **Configure Solution Path** (Optional):
   - If you want to specify a custom solution path or if auto-detection doesn't work:
   ```json
   {
     "language_servers": {
       "roslyn": {
         "enabled": true,
         "initialization_options": {
           "solution": "/absolute/path/to/your/solution.sln"
         }
       }
     }
   }
   ```
   - The path should be an absolute path to your `.sln`, `.slnx`, or `.slnf` file
   - If not specified, the extension will attempt to auto-detect the solution file

5. **Start Editing**:
   - Open any `.cs` file in your solution
   - The language server will initialize with your solution context
   - Full IDE features (go-to-definition, completions, etc.) should be available

## Debugging

The extension provides debugging support via netcoredbg, which is automatically downloaded on first use.

### Quick Start

1. **Create a debug configuration** in `.zed/debug.json` at your project root:

```json
[
  {
    "label": "Debug My App",
    "adapter": "netcoredbg",
    "config": {
      "request": "launch",
      "program": "${workspaceFolder}/bin/Debug/net9.0/MyApp.dll",
      "args": [],
      "cwd": "${workspaceFolder}",
      "stopAtEntry": false,
      "console": "internalConsole"
    },
    "build": {
      "command": "dotnet",
      "args": ["build"],
      "cwd": "${workspaceFolder}"
    }
  }
]
```

2. **Start debugging**:
   - Open the Debug panel in Zed
   - Select your debug configuration
   - Click the play button or press the debug keybinding

### Debug Configuration Reference

netcoredbg supports the following configuration options:

#### Required Fields
- `request`: Either `"launch"` (start new process) or `"attach"` (attach to existing)
- `program`: Path to the .NET DLL to debug (e.g., `"bin/Debug/net9.0/MyApp.dll"`)

#### Optional Fields
- `args`: Array of command-line arguments for the program
- `cwd`: Working directory (defaults to `${workspaceFolder}`)
- `env`: Object with environment variables (e.g., `{"VAR": "value"}`)
- `stopAtEntry`: Set to `true` to break at program entry point (default: `false`)
- `console`: Where to launch the debug target:
  - `"internalConsole"` (default) - in Zed's integrated console
  - `"integratedTerminal"` - in Zed's terminal
  - `"externalTerminal"` - in system terminal
- `justMyCode`: Only debug user code, skip framework code (default: `true`)
- `enableStepFiltering`: Skip over properties and operators (default: `true`)

#### Attach Configuration
For attaching to an existing process:
```json
{
  "request": "attach",
  "processId": 1234
}
```

### Build Tasks

The `build` field in debug configurations is optional but recommended. It tells Zed to run a build command before starting the debugger:

```json
"build": {
  "command": "dotnet",
  "args": ["build", "--configuration", "Debug"],
  "cwd": "${workspaceFolder}"
}
```

Without a build task, ensure your program is already built before debugging.

### Example Configurations

**Simple Console App:**
```json
{
  "label": "Debug Console App",
  "adapter": "netcoredbg",
  "config": {
    "request": "launch",
    "program": "${workspaceFolder}/bin/Debug/net9.0/ConsoleApp.dll",
    "cwd": "${workspaceFolder}"
  },
  "build": {
    "command": "dotnet",
    "args": ["build"]
  }
}
```

**With Arguments and Environment:**
```json
{
  "label": "Debug with Args",
  "adapter": "netcoredbg",
  "config": {
    "request": "launch",
    "program": "${workspaceFolder}/bin/Debug/net9.0/MyApp.dll",
    "args": ["--verbose", "input.txt"],
    "env": {
      "ASPNETCORE_ENVIRONMENT": "Development",
      "LOG_LEVEL": "Debug"
    },
    "cwd": "${workspaceFolder}"
  }
}
```

**Stop at Entry:**
```json
{
  "label": "Debug (Break at Start)",
  "adapter": "netcoredbg",
  "config": {
    "request": "launch",
    "program": "${workspaceFolder}/bin/Debug/net9.0/MyApp.dll",
    "stopAtEntry": true
  },
  "build": {
    "command": "dotnet",
    "args": ["build"]
  }
}
```

### Debugging Tips

1. **Set Breakpoints**: Click in the gutter (left of line numbers) to set breakpoints
2. **Inspect Variables**: Hover over variables to see their values
3. **Watch Expressions**: Add expressions to the watch panel
4. **Step Through Code**: Use step over/into/out controls
5. **View Call Stack**: See the full call stack in the debug panel

### Troubleshooting Debugging

**Debugger Won't Start:**
- Check that the `program` path is correct and the DLL exists
- Verify the build completed successfully
- Ensure the target framework matches your .NET installation

**Can't Hit Breakpoints:**
- Ensure you're building in Debug configuration (not Release)
- Check that the source code matches the built DLL
- Try setting `"justMyCode": false` to debug into framework code

**Check Debugger Logs:**
- View Zed's logs: `Help → View Logs`
- Look for lines starting with `[netcoredbg]`
- The debugger runs with `--interpreter=vscode` flag for DAP compatibility

## Architecture

### LSP Proxy Wrapper

The extension uses a wrapper binary (`roslyn-wrapper`) that acts as a proxy between Zed and the Roslyn Language Server. This is necessary because:

1. **Custom Notifications**: Zed's WASM-based extension API doesn't support sending custom LSP notifications. The wrapper intercepts the LSP communication to inject the custom `solution/open` notification.

2. **Solution Context**: After Roslyn initialization completes, the wrapper sends the `solution/open` notification to ensure Roslyn loads the entire solution and enables advanced cross-project features.

3. **Transparent Proxying**: All other LSP messages are proxied bidirectionally without modification, so the protocol remains compliant.

**Message Flow:**
```
Zed (client)
    ↓ (LSP initialize request)
Wrapper ← → Roslyn (spawned subprocess)
    ↓ (LSP initialize response)
Zed
    ↓ (solution/open notification injected by wrapper)
Wrapper → Roslyn
    ↓ (subsequent LSP messages)
Zed ← → Wrapper ← → Roslyn
```

### Solution File Detection

The extension automatically detects solution files in the workspace root with the following priority:

1. `.sln` files (traditional solution format)
2. `.slnx` files (new format introduced in Visual Studio 2022)
3. `.slnf` files (filtered solution format)

The detected solution URI is passed to Roslyn via initialization options, enabling:
- Cross-project go-to-definition
- Project-wide reference finding
- Full symbol resolution

### Initialization Options

The extension passes the following to Roslyn (via the wrapper):

```json
{
  "solution": "file:///path/to/solution.sln"
}
```

The wrapper extracts this during initialization and uses it in the `solution/open` notification.

This allows Roslyn to load the entire solution and provide accurate cross-project features.

## Troubleshooting

### Wrapper Binary Not Found

If you see errors about `roslyn-wrapper` not being found:

1. Ensure you've built the wrapper from source:
   ```bash
   cd roslyn_wrapper
   cargo build --release
   ```

2. Add the binary to your PATH:
   ```bash
   # Linux/macOS
   export PATH="/path/to/roslyn_wrapper/target/release:$PATH"
   
   # Windows - Add folder to PATH via System Settings or:
   set PATH=%PATH%;C:\path\to\roslyn_wrapper\target\release
   ```

3. Verify it's accessible:
   ```bash
   which roslyn-wrapper  # Linux/macOS
   where roslyn-wrapper  # Windows
   ```

### Language Server Won't Start

If the extension won't start, check:

1. **Wrapper binary** must be in PATH:
   ```bash
   which roslyn-wrapper
   ```

2. **Auto-download and fallback status**: The wrapper will try multiple versions and fallback mechanisms:
   - First: Check cache for any existing Roslyn version
   - Second: Try to download Roslyn 5.0.0-1.25277.114, 4.12.0, 4.11.0, 4.10.0 (in order)
   - Third: Look for globally installed Roslyn from `dotnet tool`
   - Cache location: `~/.cache/roslyn-wrapper/` (Linux/macOS) or `%LOCALAPPDATA%\roslyn-wrapper\cache\` (Windows)

3. **Internet connection**: The wrapper needs internet to download Roslyn from NuGet
   - Check your firewall settings
   - Test NuGet access: https://www.nuget.org/packages/Microsoft.CodeAnalysis.LanguageServer

**If auto-download fails completely**, you can manually install Roslyn:
```bash
dotnet tool install --global Microsoft.CodeAnalysis.LanguageServer
```

The wrapper will automatically detect and use the manually installed version instead of downloading.

### Using a Custom Roslyn Installation

You can override the Roslyn LSP path using the `ROSLYN_LSP_PATH` environment variable:

```bash
# Set the path to your custom Roslyn installation
export ROSLYN_LSP_PATH=/path/to/custom/Microsoft.CodeAnalysis.LanguageServer

# Or on Windows
set ROSLYN_LSP_PATH=C:\path\to\custom\Microsoft.CodeAnalysis.LanguageServer.exe
```

This is useful for:
- Testing a specific Roslyn version
- Using a locally built Roslyn from source
- Debugging Roslyn issues

The wrapper will use this path instead of downloading or searching for Roslyn.

### Check Wrapper Logs

The wrapper outputs diagnostic information to stderr. View Zed's logs to see wrapper output:

1. In Zed: `Help → View Logs`
2. Look for lines starting with `[roslyn_wrapper]`
3. Check for errors about Roslyn startup or initialization

### No Solution File Detected

The extension looks for `.sln`, `.slnx`, or `.slnf` files in the workspace root. If auto-detection doesn't work:

1. **Manually specify the solution path** in Zed settings (`Cmd+,` / `Ctrl+,`):

```json
{
  "language_servers": {
    "roslyn": {
      "initialization_options": {
        "solution": "/absolute/path/to/your/solution.sln"
      }
    }
  }
}
```

**Important Notes:**
- Use an **absolute path** to your solution file
- The path must point to an existing `.sln`, `.slnx`, or `.slnf` file
- On Windows, use forward slashes: `C:/Projects/MySolution.sln`
- The setting takes priority over auto-detection

2. **Verify the solution file exists** at the specified path

### Limited Features Without Solution

If no solution file is detected, Roslyn falls back to `.csproj` files in the workspace root. This provides basic features but may not include:
- Cross-project go-to-definition
- Solution-wide reference finding
- Full symbol resolution

To enable full features, place a `.sln` file in your workspace root.

### Debugging Issues

**Debugger Auto-Download Fails:**
The extension automatically downloads netcoredbg from GitHub on first use. If this fails:
- Check your internet connection
- Verify you can access: https://github.com/marcptrs/netcoredbg/releases
- Manually install netcoredbg and add to PATH (see Prerequisites)
- Check extension logs: `Help → View Logs`

**Debug Configurations Not Working:**
- Verify `.zed/debug.json` exists in your project root
- Check that `program` path points to an existing DLL
- Ensure the build task completes successfully
- Review the debug configuration schema at: `debug_adapter_schemas/netcoredbg.json`

**netcoredbg Not Found:**
The extension downloads netcoredbg automatically. If you see "not found" errors:
- Delete the cache directory to force re-download
- Manually install netcoredbg and ensure it's in PATH
- Check file permissions on the cached binary

### Debugging Not Working

Ensure netcoredbg is installed and in your PATH:

```bash
which netcoredbg  # Linux/macOS
where netcoredbg  # Windows
```

If not found, the extension will automatically download it on first debug session. If auto-download fails, install manually using the methods listed in the Prerequisites section.

## Development

### Building

```bash
cargo build --target wasm32-wasip2
```

### Testing

Load the extension in Zed during development:

1. Open Zed settings
2. Add the extension directory to `dev_extensions_path`
3. Reload Zed

### Related Projects

- **Roslyn Language Server**: https://github.com/dotnet/roslyn
- **Zed Extensions**: https://github.com/zed-industries/extensions
- **netcoredbg**: https://github.com/Samsung/netcoredbg

## License

MIT License - See LICENSE file

## Support

For issues or feature requests:
- File an issue on: https://github.com/marcptrs/csharp-roslyn/issues
- Zed bug reports: https://github.com/zed-industries/zed/issues
