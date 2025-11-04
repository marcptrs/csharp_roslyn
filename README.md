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

### Debugger (Optional)

For debugging support, install netcoredbg:

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

4. **Start Editing**:
   - Open any `.cs` file in your solution
   - The language server will initialize with your solution context
   - Full IDE features (go-to-definition, completions, etc.) should be available

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

### Check Wrapper Logs

The wrapper outputs diagnostic information to stderr. View Zed's logs to see wrapper output:

1. In Zed: `Help → View Logs`
2. Look for lines starting with `[roslyn_wrapper]`
3. Check for errors about Roslyn startup or initialization

### No Solution File Detected

The extension looks for `.sln`, `.slnx`, or `.slnf` files in the workspace root. Ensure:
- Your solution file is in the root directory of the workspace
- The filename matches one of the expected patterns

If your solution file has a non-standard name (e.g., `MySolution.sln`), you can manually specify it in Zed settings:

```json
{
  "language_servers": {
    "roslyn": {
      "initialization_options": {
        "solution": "file:///path/to/MySolution.sln"
      }
    }
  }
}
```

### Limited Features Without Solution

If no solution file is detected, Roslyn falls back to `.csproj` files in the workspace root. This provides basic features but may not include:
- Cross-project go-to-definition
- Solution-wide reference finding
- Full symbol resolution

To enable full features, place a `.sln` file in your workspace root.

### Debugging Not Working

Ensure netcoredbg is installed and in your PATH:

```bash
which netcoredbg  # Linux/macOS
where netcoredbg  # Windows
```

If not found, install using the methods listed in the Prerequisites section.

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
