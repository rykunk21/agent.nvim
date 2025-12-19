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

- **"Rust binary not found"**: Make sure Rust is installed and run the build script
- **Build fails**: Try `cargo clean && ./build.sh`
- **Plugin not loading**: Check `:messages` for errors

## Development

```bash
git clone https://github.com/your-username/agent.nvim.git
cd agent.nvim
cargo build --release
```