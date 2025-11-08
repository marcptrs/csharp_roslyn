# Testing Guide for C# Roslyn Extension

## Current Status

✅ **Completed:**
- roslyn_wrapper built and deployed to `~/Library/Application Support/Zed/extensions/work/csharp_roslyn/roslyn-wrapper/roslyn-wrapper`
- csharp_roslyn extension compiled successfully
- Extension deployed to `~/Library/Application Support/Zed/extensions/work/csharp_roslyn/csharp_roslyn.wasm`
- Settings support added for custom solution path
- Documentation updated

⏳ **Next Step:**
- Test the full flow in Zed to verify wrapper logs and solution/open notification

## Testing Steps

### 1. Configure Zed Settings

Open your Zed settings (`Cmd+,` on macOS) and add:

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

**Replace** `/absolute/path/to/your/solution.sln` with the actual path to your C# solution file.

### 2. Open a C# Project in Zed

1. Open Zed
2. Open a folder containing your C# solution
3. Open any `.cs` file in the project

### 3. Check Extension Loading

Watch for:
- Extension should load without errors
- Language server should start automatically
- Status bar should show "Roslyn" as active

### 4. Verify Wrapper Logs

Open Zed logs to see wrapper output:

**macOS:**
```bash
tail -f ~/Library/Logs/Zed/Zed.log
```

Look for these log messages:
```
[roslyn-wrapper] Starting Roslyn wrapper...
[roslyn-wrapper] Found Roslyn at: [path]
[roslyn-wrapper] Starting Roslyn LSP server...
[roslyn-wrapper] Proxying initialize request to Roslyn
[roslyn-wrapper] Received initialized notification from client
[roslyn-wrapper] Sending solution/open notification: {"solution":"file:///path/to/solution.sln"}
```

### 5. Test LSP Features

Try these features to verify the language server is working:

1. **Autocomplete**: Type `Console.` and wait for suggestions
2. **Go to Definition**: Cmd+Click (macOS) or Ctrl+Click on a symbol
3. **Find References**: Right-click on a symbol → Find References
4. **Diagnostics**: Introduce a syntax error and check for red squiggles
5. **Rename Symbol**: Right-click → Rename

### 6. Check for Errors

If something doesn't work:

1. **Check Zed logs** for errors:
   ```bash
   grep -i error ~/Library/Logs/Zed/Zed.log | tail -20
   ```

2. **Verify wrapper is running**:
   ```bash
   ps aux | grep roslyn-wrapper
   ```

3. **Check Roslyn is cached**:
   ```bash
   ls -la ~/.cache/roslyn-wrapper/
   ```

## Expected Behavior

### Successful Flow

1. Extension loads and starts wrapper
2. Wrapper checks for Roslyn in cache or downloads it
3. Wrapper starts Roslyn LSP server
4. Wrapper proxies initialize/initialized messages
5. **Wrapper sends `solution/open` notification with solution path from settings**
6. Roslyn loads the solution
7. Full LSP features available

### What You Should See

**In Zed UI:**
- "Roslyn" in the status bar (bottom right)
- No error notifications
- Autocomplete and other features working

**In Logs:**
- Wrapper startup messages
- Solution/open notification being sent
- RPC messages flowing (both send and receive)

## Configuration Examples

### Using Auto-Detection (if working directory is solution root)

```json
{
  "language_servers": {
    "roslyn": {
      "enabled": true
    }
  }
}
```

### Using Custom Solution Path

```json
{
  "language_servers": {
    "roslyn": {
      "enabled": true,
      "initialization_options": {
        "solution": "/Users/username/Projects/MyProject/MyProject.sln"
      }
    }
  }
}
```

### Using Custom Roslyn Installation

Set environment variable before starting Zed:

```bash
export ROSLYN_LSP_PATH=/path/to/custom/Microsoft.CodeAnalysis.LanguageServer
open -a Zed  # macOS
```

Or in Zed settings:

```json
{
  "terminal": {
    "env": {
      "ROSLYN_LSP_PATH": "/path/to/custom/Microsoft.CodeAnalysis.LanguageServer"
    }
  }
}
```

## Troubleshooting

### Extension Won't Load

```bash
# Check if extension is installed
ls -la ~/Library/Application\ Support/Zed/extensions/work/csharp_roslyn/

# Should show:
# - csharp_roslyn.wasm
# - roslyn-wrapper/roslyn-wrapper
```

### Wrapper Not Starting

```bash
# Test wrapper directly
~/Library/Application\ Support/Zed/extensions/work/csharp_roslyn/roslyn-wrapper/roslyn-wrapper --help

# Should output usage information
```

### No Logs Appearing

```bash
# Check Zed log file exists
ls -la ~/Library/Logs/Zed/

# Watch logs in real-time
tail -f ~/Library/Logs/Zed/Zed.log
```

### Roslyn Not Downloading

```bash
# Check internet connection
curl -I https://www.nuget.org/packages/Microsoft.CodeAnalysis.LanguageServer

# Check cache directory
mkdir -p ~/.cache/roslyn-wrapper
ls -la ~/.cache/roslyn-wrapper/
```

## Success Criteria

✅ Extension loads without errors
✅ Wrapper logs appear in Zed.log
✅ `solution/open` notification is sent with correct path
✅ Roslyn starts and responds to LSP requests
✅ Autocomplete works
✅ Go-to-definition works
✅ Diagnostics appear for errors

## Next Steps After Testing

If all tests pass:
1. Create a git commit with the changes
2. Push to GitHub repository
3. Create a release with pre-built binaries
4. Submit to Zed extensions repository

If tests fail:
1. Review error logs
2. Check wrapper and extension code
3. Debug specific issues
4. Iterate and test again
