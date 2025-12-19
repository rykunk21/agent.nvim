---
inclusion: always
---

# Development Workflow: Windows Development with Linux Testing

## Overview

This project follows a distributed development workflow where:
- **Development Environment**: Windows machine with Kiro IDE
- **Testing Environment**: Separate Linux client machine running Neovim
- **Integration**: All changes pushed to GitHub for testing on the client

## Development Process

### 1. Local Development (Windows)

**Environment Setup:**
- Use Kiro IDE on Windows for all code changes
- Work with the agent.nvim repository locally
- Make changes to Rust code, Lua scripts, and configuration files
- Test compilation and basic functionality where possible

**Key Principles:**
- Never attempt to run the Neovim plugin on the Windows development machine
- Focus on code correctness, compilation, and logical implementation
- Use the spec-driven development workflow for structured feature development
- **DO NOT IMPLEMENT OPTIONS**: When the user asks for a solution, implement exactly one approach, not multiple options to choose from

### 1.1 Development Mode

**Single Approach: Dev Branch with Local Build**
- Client uses `dev` branch with local Rust compilation
- Changes are immediately testable after push + `:Lazy sync`
- Binary detection automatically finds debug builds in `target/debug/`
- Best for rapid iteration and prototyping

### 2. Change Integration

**GitHub Workflow:**
```bash
# For development and rapid prototyping
git add .
git commit -m "dev: rapid prototype changes"
git push origin dev

# For production releases (merge dev when ready)
git checkout main
git merge dev
git push origin main
```

**Branch Strategy:**
- **`dev`**: Development branch for rapid prototyping with local builds
- **`main`** → **`plugin-dist`**: Production releases with prebuilt binaries via CI/CD

**What to Push:**
- All Rust source code changes (`src/`)
- Lua interface updates (`lua/`)
- Plugin configuration (`plugin/`)
- Build scripts (`build.sh`, `build.bat`)
- Documentation updates (`README.md`, etc.)
- Spec documents (`.kiro/specs/`)

### 3. Client Testing (Linux)

**Client Machine Setup:**
- Linux environment with Neovim installed
- Plugin installed via lazy.nvim configuration
- Rust toolchain available for local builds (optional)

**Development Configuration:**

```lua
return {
  "rykunk21/agent.nvim",
  branch = "dev",
  build = "cargo build",  -- Debug build for faster compilation
  config = function()
    require('agent').setup({
      keybindings = {
        open_agent = '<leader>ag',
        new_spec = '<leader>sn',
        open_spec = '<leader>so',
      },
    })
  end,
}
```

**Testing Process:**
1. Client uses dev branch with local debug builds
2. Client runs `:Lazy clean agent.nvim && :Lazy sync` after changes
3. Test functionality and report results
4. Issues communicated back to development environment

**Test Commands on Client:**
```bash
# Check plugin status
:checkhealth agent

# Test basic functionality (note: changed keybinding)
<leader>ag  # Should open agent interface
<leader>sn  # Should create new spec
<leader>so  # Should open existing spec

# Check for errors
:messages

# Debug paths
:lua require('agent').debug_paths()
```

## Communication Protocol

### Issue Reporting Format

When the client encounters issues, report them with:

1. **Error Message**: Exact error text from Neovim
2. **Context**: What action triggered the error
3. **Environment**: Neovim version, OS details
4. **Logs**: Output from `:messages` or `:checkhealth agent`

### Debug Information Requests

When requesting debug information from client:
- Be specific about what commands to run
- Ask for exact output, not summaries
- Request file existence checks when needed
- Ask for build script output if compilation issues suspected

## Development Guidelines

### Code Changes

**Before Pushing:**
- Ensure Rust code compiles (use `cargo check` if available)
- Verify Lua syntax is correct
- Update documentation if interfaces change
- Test build scripts on Windows if possible

**Rust Development:**
- Focus on implementing the spec tasks in order
- Ensure all modules are properly exported in `mod.rs` files
- Use proper error handling with `PluginResult` type
- Follow the established architecture patterns

**Lua Interface:**
- Maintain compatibility between `lua/agent/` and `lua/nvim-spec-agent/`
- Provide clear error messages for missing binaries
- Include helpful troubleshooting information in notifications

### Build System

**Build Script Requirements:**
- Must work on both Linux and Windows
- Should provide clear success/failure feedback
- Must copy binaries to expected locations (`bin/` directory)
- Should validate that Rust toolchain is available

**Binary Locations:**
The Lua code searches for binaries in this order:
1. `plugin_dir/bin/nvim-spec-agent[.exe]`
2. `plugin_dir/target/release/nvim-spec-agent[.exe]`
3. `nvim-spec-agent` in system PATH

## Troubleshooting Common Issues

### "Rust Binary Not Found"

**Likely Causes:**
- Build script failed during installation
- Rust toolchain not available on client
- Binary not copied to expected location

**Debug Steps:**
1. Check if Rust is installed: `cargo --version`
2. Manually run build script: `./build.sh`
3. Verify binary exists: `ls -la bin/` and `ls -la target/release/`
4. Check file permissions: `ls -la bin/nvim-spec-agent`

### "Agent Backend Not Initialized"

**Likely Causes:**
- Binary exists but fails to start
- Communication protocol issues
- Missing dependencies

**Debug Steps:**
1. Try running binary manually: `./bin/nvim-spec-agent`
2. Check for runtime dependencies
3. Verify JSON communication protocol

### Health Check Function Errors

**Likely Causes:**
- Health check function in wrong file location
- Vim script syntax errors

**Debug Steps:**
1. Check if `autoload/health/agent.vim` exists
2. Verify function is not duplicated in `plugin/agent.vim`
3. Test health check: `:checkhealth agent`

## File Organization

### Critical Files for Client Testing

**Must Work:**
- `build.sh` / `build.bat` - Build scripts
- `lua/agent/init.lua` - Main Lua interface
- `plugin/agent.vim` - Plugin registration
- `autoload/health/agent.vim` - Health checks
- `src/main.rs` - Rust binary entry point

**Configuration:**
- `Cargo.toml` - Rust dependencies and build config
- `README.md` - Installation and usage instructions

### Development-Only Files

**Not Critical for Client:**
- `.kiro/specs/` - Development specifications
- `.kiro/steering/` - Development guidelines
- Development tools and scripts

## Success Criteria

### Minimum Viable Plugin

For basic functionality, the client should be able to:
1. Install plugin without errors
2. Run `:checkhealth agent` successfully
3. Execute `<leader>ag` without "binary not found" errors
4. See floating window appear when toggling agent

### Current Status (Working)

✅ **Prebuilt Binary System**: CI/CD builds and distributes binaries automatically
✅ **Platform Detection**: Correctly identifies and uses Linux x64 binary
✅ **Rust Backend Communication**: JSON communication between Lua and Rust working
✅ **Window Creation**: Floating windows appear when agent is toggled
✅ **Keybinding Resolution**: Changed to `<leader>ag` to avoid timeout conflicts

### Full Functionality (In Progress)

For complete functionality:
1. Dual window interface with chat history and input
2. Can create and navigate specs through UI
3. Agent communication works bidirectionally with responses
4. All keybindings function reliably

### Development Workflow Status

✅ **Production Pipeline**: `main` → `plugin-dist` for stable releases
✅ **Development Pipeline**: `dev` → `plugin-dev` for rapid prototyping
✅ **Local Development**: Direct builds from `main` for fastest iteration

## Version Control Strategy

### Commit Messages

Use descriptive commit messages that help the client understand what changed:
- `fix: resolve health check function placement error`
- `feat: improve binary detection with better error messages`
- `build: enhance build scripts with progress indicators`
- `docs: update installation instructions`

### Branch Strategy

- Use `main` branch for stable, testable changes
- Create feature branches for major changes if needed
- Always ensure `main` branch is in a testable state

### Release Process

When ready for client testing:
1. Ensure all changes are committed and pushed
2. Notify client that updates are available
3. Client runs `:Lazy sync` to pull changes
4. Client tests and reports results
5. Iterate based on feedback