# agent.nvim

An enhanced Neovim plugin with spec-driven development capabilities, built in Rust for performance and reliability.

> **⚠️ Development Status**: This plugin is currently in active development. Core functionality is implemented but some features may be incomplete. Use at your own discretion.

## Features

- 🚀 **Spec-driven Development**: Transform ideas into requirements, design, and implementation tasks
- 💬 **Agent Interface**: Two-window chat interface for AI assistance
- 🔧 **Command Execution**: Safe command approval workflow with visual feedback
- 📝 **Visual Operation Blocks**: Real-time progress indicators for file operations
- ⚙️ **Configuration Management**: Persistent settings and conversation history
- 🏗️ **Rust Backend**: High-performance backend for responsive UI

## Requirements

- Neovim 0.5.0+
- Rust (for building the binary)
- Git

## Installation

### Using lazy.nvim

1. Add this to your `~/.config/nvim/lua/plugins/agent.lua`:

```lua
return {
  "rykunk21/agent.nvim",
  -- Build with permission fix (recommended for Linux/Mac)
  build = function(plugin)
    -- Fix permissions and build
    vim.fn.system('chmod +x ' .. plugin.dir .. '/build.sh')
    local build_cmd = vim.fn.has('win32') == 1 and 'build.bat' or './build.sh'
    local result = vim.fn.system('cd ' .. plugin.dir .. ' && ' .. build_cmd)
    if vim.v.shell_error ~= 0 then
      vim.notify('Build failed: ' .. result, vim.log.levels.ERROR)
    else
      vim.notify('Build completed successfully!', vim.log.levels.INFO)
    end
  end,
  config = function()
    require('agent').setup({
      keybindings = {
        open_agent = '<leader>sa',  -- Open spec agent
        new_spec = '<leader>sn',    -- Create new spec
        open_spec = '<leader>so',   -- Open existing spec
      },
      ui = {
        border_style = 'rounded',
        window_width_ratio = 0.8,
        window_height_ratio = 0.6,
      },
    })
  end,
  cmd = { 'SpecAgent', 'SpecNew', 'SpecOpen' },
  keys = {
    { '<leader>sa', desc = 'Open Spec Agent' },
    { '<leader>sn', desc = 'New Spec' },
    { '<leader>so', desc = 'Open Spec' },
  },
}
```

2. Restart Neovim and run `:Lazy sync`

### Manual Installation

1. Clone the repository:
```bash
git clone https://github.com/rykunk21/agent.nvim.git ~/.local/share/nvim/site/pack/plugins/start/agent.nvim
```

2. Build the Rust binary:
```bash
cd ~/.local/share/nvim/site/pack/plugins/start/agent.nvim
./build.sh  # On Linux/macOS
# or
build.bat   # On Windows
```

3. Add the setup to your `init.lua`:
```lua
require('agent').setup()
```

## Usage

### Basic Commands

- `:SpecAgent` - Open the spec agent interface
- `:SpecNew [name]` - Create a new spec
- `:SpecOpen [name]` - Open an existing spec
- `:SpecClose` - Close the agent interface

### Default Keybindings

- `<leader>sa` - Open spec agent
- `<leader>sn` - Create new spec
- `<leader>so` - Open existing spec

### Spec-Driven Development Workflow

1. **Create a new spec**: Use `:SpecNew feature-name` or `<leader>sn`
2. **Requirements Phase**: Define user stories and acceptance criteria
3. **Design Phase**: Create architecture and correctness properties
4. **Tasks Phase**: Generate actionable implementation tasks
5. **Implementation**: Execute tasks with agent assistance

## Configuration

The plugin can be configured through the `setup()` function:

```lua
require('agent').setup({
  -- Keybindings
  keybindings = {
    open_agent = '<leader>sa',
    new_spec = '<leader>sn',
    open_spec = '<leader>so',
    close_agent = '<Esc>',
  },
  
  -- UI settings
  ui = {
    border_style = 'rounded', -- 'single', 'double', 'rounded', 'solid', 'shadow'
    window_width_ratio = 0.8,
    window_height_ratio = 0.6,
  },
  
  -- Auto-start the backend when Neovim starts
  auto_start = false,
  
  -- Custom binary path (auto-detected by default)
  rust_binary_path = nil,
})
```

## Development

### Building from Source

1. Clone the repository
2. Install Rust: https://rustup.rs/
3. Build the project:
```bash
cargo build --release
```

### Running Tests

```bash
cargo test
```

### Project Structure

```
agent.nvim/
├── src/
│   ├── main.rs              # Binary entry point
│   ├── lib.rs               # Library entry point
│   ├── agent/               # Agent functionality
│   ├── config/              # Configuration management
│   ├── spec/                # Spec-driven development
│   ├── ui/                  # UI components
│   └── utils/               # Utilities
├── lua/
│   └── agent/               # Lua interface
├── plugin/                  # Vim plugin files
├── build.sh                 # Unix build script
├── build.bat                # Windows build script
└── Cargo.toml               # Rust configuration
```

## Troubleshooting

### Binary Not Found

If you get "Rust binary not found" errors:

1. Make sure Rust is installed: `cargo --version`
2. Fix permissions and run build script:
   ```bash
   cd ~/.local/share/nvim/lazy/agent.nvim  # or your plugin directory
   chmod +x build.sh
   ./build.sh
   ```
3. Check that the binary exists in `bin/nvim-spec-agent`

### Permission Denied

If you get "Permission denied" when running `./build.sh`:

```bash
chmod +x build.sh
./build.sh
```

### Build Failures

1. Update Rust: `rustup update`
2. Clean and rebuild: `cargo clean && cargo build --release`
3. Check that all dependencies are available

### Plugin Not Loading

1. Check Neovim version: `:version` (requires 0.5.0+)
2. Verify plugin installation: `:Lazy` (for lazy.nvim users)
3. Check for errors: `:messages`

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## License

MIT License - see LICENSE file for details.