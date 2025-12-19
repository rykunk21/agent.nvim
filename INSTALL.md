# Quick Installation Guide

## Prerequisites

- Neovim 0.5.0+
- Rust toolchain (install from [rustup.rs](https://rustup.rs/))
- Git

## Installation with lazy.nvim

1. Add to your `~/.config/nvim/lua/plugins/agent.lua`:

```lua
return {
  "your-username/agent.nvim", -- Replace with your GitHub username
  build = function()
    if vim.fn.has('win32') == 1 or vim.fn.has('win64') == 1 then
      vim.fn.system('build.bat')
    else
      vim.fn.system('./build.sh')
    end
  end,
  config = function()
    require('agent').setup({
      keybindings = {
        open_agent = '<leader>sa',
        new_spec = '<leader>sn',
        open_spec = '<leader>so',
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

3. Test with `<leader>sa`

## Manual Installation

```bash
git clone https://github.com/your-username/agent.nvim.git ~/.local/share/nvim/site/pack/plugins/start/agent.nvim
cd ~/.local/share/nvim/site/pack/plugins/start/agent.nvim
./build.sh  # or build.bat on Windows
```

## Troubleshooting

### "Rust binary not found" Error

This means the Rust binary wasn't built during installation. Try:

1. **Check if Rust is installed**: `cargo --version`
2. **Manually run build script**:
   - Linux/macOS: `cd ~/.local/share/nvim/lazy/agent.nvim && ./build.sh`
   - Windows: `cd %LOCALAPPDATA%\nvim-data\lazy\agent.nvim && build.bat`
3. **Check binary exists**: Look for `bin/nvim-spec-agent` or `bin/nvim-spec-agent.exe`

### "Agent backend not initialized" Error

This happens when the Rust binary exists but fails to start:

1. **Test binary manually**: `./bin/nvim-spec-agent` (should not exit immediately)
2. **Check Neovim logs**: `:messages` for error details
3. **Enable debug logging**: Add `log_level = 'debug'` to your config

### Build Fails

1. **Clean and rebuild**: `cargo clean && ./build.sh`
2. **Check Rust version**: Requires Rust 1.70+
3. **Missing dependencies**: Install build tools for your platform

### Health Check

Run `:checkhealth agent` to diagnose issues.

## Development

```bash
git clone https://github.com/your-username/agent.nvim.git
cd agent.nvim
cargo build --release
```