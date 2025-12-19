-- Example lazy.nvim configuration for agent.nvim
-- Place this in your ~/.config/nvim/lua/plugins/agent.lua

return {
  "rykunk21/agent.nvim",
  -- No build step needed! Prebuilt binaries included
  config = function()
    require('agent').setup({
      -- Optional configuration
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
      auto_start = false, -- Set to true to auto-start the backend
    })
  end,
  -- Optional: specify dependencies if needed
  dependencies = {
    'nvim-lua/plenary.nvim', -- If you use plenary functions
  },
  -- Optional: lazy load the plugin
  cmd = { 'SpecAgent', 'SpecNew', 'SpecOpen' },
  keys = {
    { '<leader>sa', desc = 'Open Spec Agent' },
    { '<leader>sn', desc = 'New Spec' },
    { '<leader>so', desc = 'Open Spec' },
  },
}