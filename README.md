# C# Language Server Extension for Zed

A fast, modern C# language support extension for Zed editor using Microsoft's Roslyn Language Server via proxy.

## Features

- ⚡ **Fast Startup**: 2-3 seconds (cached)
- 🎯 **Modern LSP**: Direct integration with Microsoft's official Roslyn LSP server
- 🎨 **Semantic Highlighting**: Advanced syntax highlighting using Roslyn's semantic model
- 💡 **IntelliSense**: Full code completion with documentation
- 🔍 **Navigation**: Go to definition, find references, workspace symbols
- ⚠️ **Diagnostics**: Real-time errors, warnings, and suggestions
- 🔧 **Code Actions**: Quick fixes, refactorings, and code generation
- 📝 **Formatting**: Document and range formatting with .editorconfig support
- 🏷️ **Inlay Hints**: Parameter names and type hints

## Installation

1. Clone this repository:
   ```bash
   git clone https://github.com/marcptrs/csharp_roslyn.git
   cd csharp_roslyn
   ```

2. Install as dev extension in Zed:
   - Open Zed
   - Press `Cmd+Shift+P` (macOS) or `Ctrl+Shift+P` (Linux/Windows)
   - Type "zed: install dev extension"
   - Select the cloned `csharp_roslyn` directory

## Requirements

- **Zed Editor**: Version 0.199.0 or later (for debugger support)
- **.NET SDK**: Version 6.0 or later
  - Download from: https://dotnet.microsoft.com/download
- **Roslyn Language Server**: Auto-downloaded from NuGet on first use (or configure manually)

## Quick Start

### 1. Install .NET SDK
```bash
dotnet --version
# Should show 6.0.0 or higher
# If not installed, download from: https://dotnet.microsoft.com/download
```

### 2. Open a C# Project

Open any folder containing `.sln` or `.csproj` files. On first use:
- The extension will automatically download the Roslyn Language Server from NuGet
- IntelliSense will work automatically once the download completes

### 3. (Optional) Manual Server Configuration

If you prefer to use an existing Roslyn installation or the auto-download fails, you can configure the server path manually.

If you have Visual Studio Code with the C# extension installed:

```bash
ls ~/.vscode/extensions/ms-dotnettools.csharp-*/.roslyn/Microsoft.CodeAnalysis.LanguageServer.dll
```

This will show your Roslyn installation path, for example:
```
/Users/yourusername/.vscode/extensions/ms-dotnettools.csharp-2.90.60-darwin-arm64/.roslyn/Microsoft.CodeAnalysis.LanguageServer.dll
```

Then add the server path to your Zed settings (`Cmd+,` or Zed > Settings):

```json
{
  "lsp": {
    "roslyn": {
      "initialization_options": {
        "serverPath": "/Users/yourusername/.vscode/extensions/ms-dotnettools.csharp-2.90.60-darwin-arm64/.roslyn/Microsoft.CodeAnalysis.LanguageServer.dll"
      }
    }
  }
}
```

Replace the path with your actual path.

## Configuration

By default, the extension automatically downloads the Roslyn Language Server from NuGet on first use. All settings are optional.

```json
{
  "lsp": {
    "roslyn": {
      "binary": {
        "path": "/usr/local/share/dotnet/dotnet"  // Custom .NET SDK path (if installed in non-standard location)
      },
      "initialization_options": {
        "version": "5.0.0-1.25277.114",  // Specific Roslyn version to download from NuGet (default: 5.0.0-1.25277.114)
        "serverPath": "/path/to/Microsoft.CodeAnalysis.LanguageServer.dll",  // Manual server path (skips auto-download)
        "enableImportCompletion": true,  // Suggest types from unimported namespaces and auto-add using statements
        "enableDecompilationSupport": true,  // Navigate to .NET Framework source code via decompilation
        "enableAnalyzersSupport": false,  // Enable Roslyn analyzers for code quality suggestions
        "organizeImportsOnFormat": false,  // Sort and remove unused using statements when formatting
        "enableEditorConfigSupport": false  // Respect .editorconfig files for formatting rules
      }
    }
  }
}
```

## Solution File Support

The extension automatically detects solution files (`.sln`) in your workspace for better IntelliSense.

### Automatic Detection
The extension will search for solution files in this order:
1. **Workspace root**: `{workspace_folder_name}.sln` (e.g., `MyProject.sln` in folder `MyProject`)
2. **Workspace root**: `solution.sln` or `Solution.sln`
3. **Subdirectories** (up to 2 levels deep): Searches for `{subdirectory_name}.sln` in subdirectories

**Example**: If your workspace is opened at the parent directory:
```
MyWorkspace/
├── ProjectA/
│   └── ProjectA.sln
├── ProjectB/
│   └── ProjectB.sln
└── nested/
    └── ProjectC/
        └── ProjectC.sln
```
The extension will automatically find and use the first solution file alphabetically (`ProjectA.sln`).

**Recommendation**: For best results, open the specific project directory in Zed rather than the parent directory.

If found, the solution file is automatically passed to the language server.

### Multi-Project Solutions
✅ **Fully supported!** The extension works seamlessly with solutions containing multiple projects:
- All projects load automatically
- Cross-project references work correctly
- BCL navigation and decompilation work in all projects
- No configuration needed

**Example**:
```bash
MyMultiProjectSolution/
├── MyMultiProjectSolution.sln
├── WebApp/
│   └── WebApp.csproj
├── BusinessLogic/
│   └── BusinessLogic.csproj
└── DataAccess/
    └── DataAccess.csproj
```

### Create a Solution File
If you don't have a solution file:

```bash
# Navigate to your project directory
cd /path/to/your/project

# Create a solution file (replace 'MyProject' with your project name)
dotnet new sln -n MyProject

# Add your .csproj file(s) to the solution
dotnet sln add MyProject.csproj

# For multiple projects:
dotnet sln add **/*.csproj

# Restore and build
dotnet restore
dotnet build
```

## Features in Detail

### Code Completion

Trigger completion with:
- `.` after an object or type
- `Ctrl+Space` anywhere
- `<` for generic types
- `override` keyword for method overrides

**Example**:
```csharp
var list = new List<string>();
list. // Shows: Add, Remove, Count, Clear, etc.

class MyClass : BaseClass {
    override // Shows overrideable methods from BaseClass
}
```

### Semantic Highlighting

Advanced syntax coloring based on Roslyn's semantic model:
- **Classes**: Different color from interfaces and structs
- **Static members**: Visually distinct from instance members
- **Parameters**: Different from local variables
- **Deprecated symbols**: Strikethrough styling

**Customization**: Colors are controlled by your Zed theme.

### Diagnostics

- **Real-time diagnostics**: Errors appear as you type
- More immediate feedback
- Higher resource usage

**Severity Levels**:
- 🔴 **Error**: Prevents compilation
- 🟡 **Warning**: Compiles but may cause issues
- 🔵 **Info**: Suggestions for improvement
- ⚪ **Hint**: Style and convention suggestions

### Code Actions

Quick fixes and refactorings:
- **Add using statement**: For unresolved types
- **Generate constructor**: Create constructor from fields
- **Extract method**: Convert code selection to method
- **Rename symbol**: Rename across entire workspace
- **Organize imports**: Sort and remove unused usings

**Trigger**: Light bulb icon or `Cmd+.` (macOS) / `Ctrl+.` (Windows/Linux)

### Inlay Hints

Shows additional context inline:
- **Parameter names**: `Calculate(first: 10, second: 20)`
- **Type hints**: `var message: string = "Hello"`

**Toggle**: Via Zed settings:
```json
{
  "inlay_hints": {
    "enabled": true
  }
}
```

## Supported .NET Versions

| .NET Version | Supported |
|--------------|-----------|
| .NET 8.0 | ✅ Fully supported |
| .NET 7.0 | ✅ Fully supported |
| .NET 6.0 | ✅ Fully supported (LTS) |
| .NET 5.0 | ⚠️ Works but EOL |
| .NET Core 3.1 | ❌ Not supported |
| .NET Framework | ❌ Use .NET 6+ SDK |

**Recommendation**: Use .NET 6.0 (LTS) or .NET 8.0 (latest LTS)

## Project Types

| Project Type | Support Status |
|--------------|----------------|
| Console App | ✅ Full support |
| Class Library | ✅ Full support |
| ASP.NET Core | ✅ Full support |
| Blazor | ⚠️ C# only (Razor support to be developed) |
| MAUI | ⚠️ C# only (XAML support to be developed) |

## Known Limitations

1. **Large Solutions**: Projects with 1000+ files may have slower analysis
2. **Framework Navigation**: Requires decompilation (slight delay on first access)
3. **Global Tools**: Roslyn analyzers from global tools not supported
4. **Multi-Targeting**: Only primary target framework analyzed

## Architecture

### Components

```
csharp_roslyn/
├── src/
│   ├── lib.rs              # Extension entry point
│   ├── csharp.rs           # LSP configuration and command building
│   ├── nuget.rs            # NuGet package download and management
│   └── debugger.rs         # Debug adapter (netcoredbg) setup
├── proxy/                  # LSP proxy for protocol translation
│   ├── src/
│   │   ├── main.rs         # Proxy entry point
│   │   ├── connection.rs   # LSP connection handling
│   │   ├── router.rs       # Message routing
│   │   ├── id_mapper.rs    # Request ID mapping
│   │   ├── message.rs      # LSP message types
│   │   └── middleware/     # Protocol middleware
│   │       ├── initialization.rs       # Server initialization
│   │       ├── diagnostics.rs          # Diagnostic handling
│   │       ├── solution_loader.rs      # Solution file loading
│   │       ├── document_lifecycle.rs   # Document sync
│   │       ├── inlay_hints.rs          # Inlay hints support
│   │       ├── configuration.rs        # Configuration sync
│   │       ├── capability_registration.rs  # Dynamic capabilities
│   │       ├── project_restore.rs      # Project restore handling
│   │       ├── refresh.rs              # Workspace refresh
│   │       ├── definition_logger.rs    # Definition logging (debug)
│   │       └── custom.rs               # Custom notifications
│   └── Cargo.toml
├── languages/              # Tree-sitter grammar configuration
├── debug_adapter_schemas/  # DAP schemas for netcoredbg
├── tests/                  # Integration tests
└── Cargo.toml
```

### How It Works

1. **Extension Layer** (`src/`): Integrates with Zed, downloads Roslyn from NuGet, and spawns the proxy
2. **Proxy Layer** (`proxy/`): Translates LSP protocol between Zed and Roslyn, handles middleware for features
3. **Roslyn Server**: Microsoft's official C# language server (downloaded automatically)
4. **Debug Adapter**: netcoredbg for debugging support (downloaded automatically)

## Contributing

Contributions welcome! Please:
1. File issues for bugs or feature requests
2. Submit PRs with tests
3. Follow existing code style
4. Update documentation

## License

This extension is MIT licensed. See LICENSE file.

The Roslyn Language Server is licensed under the MIT license by Microsoft.

## Credits

- **Roslyn Language Server**: Microsoft
- **Extension**: Zed Community
- **Zed Editor**: Zed Industries

## Support

- **Issues**: https://github.com/marcptrs/csharp_roslyn/issues
- **Discussions**: https://github.com/marcptrs/csharp_roslyn/discussions

## Changelog

### v0.0.1 (Initial Release)
- ✨ Initial implementation with Roslyn LSP and embedded proxy
- ⚡ Fast startup via caching
- 🎨 Semantic highlighting support
- 💡 Full IntelliSense and diagnostics
- 🔧 Code actions and refactoring
- 📝 .editorconfig support
- 🏷️ Inlay hints for parameters and types
- 🔍 Automatic solution file detection
- 🎯 Multi-project solution support
- 🐛 BCL navigation and decompilation support
