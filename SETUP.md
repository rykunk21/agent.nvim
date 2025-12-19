# agent.nvim Setup Instructions

## Repository Setup

Your plugin is now configured to work with the GitHub repository:
**`git@github.com:rykunk21/agent.nvim.git`**

## Quick Start for Linux Testing

### 1. Push to GitHub (from Windows)
```bash
git add .
git commit -m "Complete agent.nvim plugin implementation"
git push origin main
```

### 2. Install on Linux
Create `~/.config/nvim/lua/plugins/agent.lua`:

```lua
return {
  "rykunk21/agent.nvim",
  build = "./build.sh",
  config = function()
    require('agent').setup({
      keybindings = {
        open_agent = '<leader>af',  -- Match your current binding
        new_spec = '<leader>sn',
        open_spec = '<leader>so',
      },
      ui = {
        border_style = 'rounded',
        window_width_ratio = 0.75,
        window_height_ratio = 0.75,
      },
    })
  end,
  cmd = { 'SpecAgent', 'SpecNew', 'SpecOpen' },
  keys = {
    { '<leader>af', desc = 'Open Agent Interface' },
    { '<leader>sn', desc = 'New Spec' },
    { '<leader>so', desc = 'Open Spec' },
  },
}
```

### 3. Test Installation
```bash
# Start Neovim
nvim

# Install the plugin
:Lazy sync

# Test the agent interface
<leader>af
```

## Available Commands

- `:SpecAgent` - Open the agent interface
- `:SpecNew [name]` - Create a new spec
- `:SpecOpen [name]` - Open existing spec
- `:SpecClose` - Close the agent interface
- `:SpecStatus` - Show plugin status

## Default Keybindings

- `<leader>af` - Open agent interface (matches your current setup)
- `<leader>sn` - Create new spec
- `<leader>so` - Open existing spec

## File Structure

```
agent.nvim/
├── src/
│   ├── main.rs              # Binary entry point
│   ├── lib.rs               # Library
│   └── ...                  # All your Rust modules
├── lua/
│   └── agent/               # Lua interface
│       └── init.lua
├── plugin/
│   └── agent.vim            # Vim plugin registration
├── bin/
│   └── nvim-spec-agent.exe  # Compiled binary
├── build.sh                 # Linux build script
├── build.bat                # Windows build script
└── README.md                # Documentation
```

## Development Workflow

1. **Develop on Windows** with Kiro IDE
2. **Commit and push** changes to GitHub
3. **Pull on Linux** - lazy.nvim will auto-rebuild
4. **Test** with `<leader>af`

## Troubleshooting

### Binary Not Found
```bash
# Check if Rust is installed
cargo --version

# Manually build if needed
cd ~/.local/share/nvim/lazy/agent.nvim
./build.sh
```

### Plugin Not Loading
```bash
# Check Neovim version (needs 0.5.0+)
nvim --version

# Check for errors
:messages

# Check plugin status
:Lazy
```

## Next Steps

1. Push this code to your GitHub repository
2. Test the installation on your Linux machine
3. Iterate and improve based on your usage

The plugin is now fully self-contained and ready for distribution!