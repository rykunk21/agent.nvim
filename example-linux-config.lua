-- Example configuration for your Linux dotfiles
-- Place this in ~/.config/nvim/lua/plugins/agent.lua

return {
  "rykunk21/agent.nvim",
  build = "./build.sh",
  config = function()
    require('agent').setup({
      keybindings = {
        open_agent = '<leader>af',  -- Match your current custom-window.lua binding
        new_spec = '<leader>sn',
        open_spec = '<leader>so',
      },
      ui = {
        border_style = 'rounded',
        window_width_ratio = 0.75,  -- Match your current window size
        window_height_ratio = 0.75,
      },
      auto_start = false,
    })
  end,
  cmd = { 'SpecAgent', 'SpecNew', 'SpecOpen' },
  keys = {
    { '<leader>af', desc = 'Open Agent Interface' },
    { '<leader>sn', desc = 'New Spec' },
    { '<leader>so', desc = 'Open Spec' },
  },
}