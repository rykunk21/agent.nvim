-- Agent.nvim Lua Interface
local M = {}

-- Load modules
local communication = require('agent.communication')
local context = require('agent.context')
local ui = require('agent.ui')

-- Plugin state
local state = {
  initialized = false,
  rust_job_id = nil,
  windows = {},
  current_spec = nil,
  chat_history = {},
  dual_window_open = false,
  grpc_connected = false,
  pending_requests = {},
}

-- Configuration
local default_config = {
  auto_start = false,
  rust_binary_path = nil,
  log_level = 'info',
  ui = {
    border_style = 'rounded',
    window_width_ratio = 0.8,
    window_height_ratio = 0.6,
  },
}

local config = default_config

-- Get the plugin directory
local function get_plugin_dir()
  local str = debug.getinfo(1, "S").source:sub(2)
  local script_dir = vim.fn.fnamemodify(str, ':h')
  return vim.fn.fnamemodify(script_dir, ':h:h')
end

-- Find the prebuilt binary for current platform
local function find_rust_binary()
  if config.rust_binary_path then
    return config.rust_binary_path
  end
  
  local plugin_dir = get_plugin_dir()
  
  local selector_path = plugin_dir .. '/bin/select-binary.lua'
  if vim.fn.filereadable(selector_path) == 1 then
    local selector = dofile(selector_path)
    local binary_path = selector.get_binary_path(plugin_dir)
    if binary_path then
      return binary_path
    end
  end
  
  local fallback_paths = {
    plugin_dir .. '/bin/nvim-spec-agent',
    plugin_dir .. '/bin/nvim-spec-agent.exe',
    plugin_dir .. '/target/debug/nvim-spec-agent',
    plugin_dir .. '/target/debug/nvim-spec-agent.exe',
    plugin_dir .. '/target/release/nvim-spec-agent',
    plugin_dir .. '/target/release/nvim-spec-agent.exe',
    'nvim-spec-agent',
  }
  
  for _, path in ipairs(fallback_paths) do
    if vim.fn.executable(path) == 1 then
      return path
    end
  end
  
  return nil
end

-- Setup function
function M.setup(user_config)
  config = vim.tbl_deep_extend('force', default_config, user_config or {})
  
  config.rust_binary_path = find_rust_binary()
  
  if not config.rust_binary_path then
    vim.notify('agent.nvim: Binary not found, will search when needed', vim.log.levels.WARN)
    if vim.fn.executable('cargo') == 0 then
      vim.notify('Install Rust from: https://rustup.rs/', vim.log.levels.WARN)
    end
  end
  
  -- Initialize UI module
  ui.init(config.ui)
  
  -- Initialize communication module
  communication.init('localhost:50051')
  
  -- Register commands
  vim.api.nvim_create_user_command('AgentToggle', function()
    M.toggle_window()
  end, { desc = 'Toggle Agent Interface' })
  
  vim.api.nvim_create_user_command('AgentOpen', function()
    M.open_agent()
  end, { desc = 'Open Agent Interface' })
  
  vim.api.nvim_create_user_command('AgentClose', function()
    M.close_dual_window()
  end, { desc = 'Close Agent Interface' })
  
  vim.api.nvim_create_user_command('AgentStatus', function()
    M.show_status()
  end, { desc = 'Show Agent Status' })
  
  if config.keybindings and config.keybindings.open_agent then
    vim.schedule(function()
      if config.keybindings.open_agent then
        vim.keymap.set('n', config.keybindings.open_agent, M.toggle_window, { 
          desc = 'Toggle Spec Agent',
          noremap = true,
          silent = true
        })
      end
      
      if config.keybindings.new_spec then
        vim.keymap.set('n', config.keybindings.new_spec, M.new_spec, { 
          desc = 'New Spec',
          noremap = true,
          silent = true
        })
      end
      
      if config.keybindings.open_spec then
        vim.keymap.set('n', config.keybindings.open_spec, M.open_spec, { 
          desc = 'Open Spec',
          noremap = true,
          silent = true
        })
      end
    end)
  end
  
  if config.auto_start and config.rust_binary_path then
    M.start_rust_backend()
  end
end

-- Start the Rust backend
function M.start_rust_backend()
  if state.rust_job_id then
    return true
  end
  
  if not config.rust_binary_path then
    return false
  end
  
  local cmd = { config.rust_binary_path }
  
  state.rust_job_id = vim.fn.jobstart(cmd, {
    on_stdout = function(_, data, _)
      M.handle_rust_output(data)
    end,
    on_stderr = function(_, data, _)
      M.handle_rust_error(data)
    end,
    on_exit = function(_, code, _)
      M.handle_rust_exit(code)
    end,
    stdin = 'pipe',
  })
  
  if state.rust_job_id <= 0 then
    return false
  end
  
  state.initialized = true
  
  vim.defer_fn(function()
    M.send_to_rust({ type = 'ping' })
  end, 100)
  
  return true
end

-- Stop the Rust backend
function M.stop_rust_backend()
  if state.rust_job_id then
    vim.fn.jobstop(state.rust_job_id)
    state.rust_job_id = nil
    state.initialized = false
  end
end

-- Handle Rust backend output
function M.handle_rust_output(data)
  for _, line in ipairs(data) do
    if line and line ~= '' then
      local ok, message = pcall(vim.json.decode, line)
      if ok then
        M.handle_rust_message(message)
      end
    end
  end
end

-- Handle Rust backend errors
function M.handle_rust_error(data)
  for _, line in ipairs(data) do
    if line and line ~= '' then
      -- Ignore stderr for now
    end
  end
end

-- Handle Rust backend exit
function M.handle_rust_exit(code)
  state.rust_job_id = nil
  state.initialized = false
  
  if code ~= 0 then
    vim.notify('Rust backend exited with code: ' .. code, vim.log.levels.WARN)
  end
end

-- Handle messages from Rust backend
function M.handle_rust_message(message)
  local msg_type = message.type
  
  if msg_type == 'window_create' then
    M.handle_window_create(message.data)
  elseif msg_type == 'window_update' then
    M.handle_window_update(message.data)
  elseif msg_type == 'chat_response' then
    M.handle_chat_response(message.data)
  elseif msg_type == 'notification' then
    vim.notify(message.data.text, message.data.level)
  elseif msg_type == 'spec_update' then
    M.handle_spec_update(message.data)
  end
end

-- Toggle window
function M.toggle_window()
  state.dual_window_open = not state.dual_window_open
  if state.dual_window_open then
    M.draw_dual_window()
  else
    M.close_dual_window()
  end
end

-- Draw dual window
function M.draw_dual_window()
  if not config.rust_binary_path then
    config.rust_binary_path = find_rust_binary()
    if not config.rust_binary_path then
      -- Continue silently
    end
  end
  
  ui.create_interface()
  
  if config.rust_binary_path and not state.initialized then
    M.start_rust_backend()
  end
  
  if state.initialized then
    M.send_to_rust({ type = 'open_agent' })
  end
  
  state.dual_window_open = true
end

-- Close dual window
function M.close_dual_window()
  ui.close_interface()
  state.windows = {}
  state.dual_window_open = false
  
  if state.initialized then
    M.send_to_rust({ type = 'close_agent' })
  end
end

-- Legacy aliases
M.close_agent_interface = M.close_dual_window
M.toggle_agent = M.toggle_window

-- Open agent interface
function M.open_agent()
  if not state.dual_window_open then
    state.dual_window_open = true
    M.draw_dual_window()
  end
end

-- Create new spec
function M.new_spec(feature_name)
  if not state.initialized then
    vim.notify('Agent backend not available. Spec creation requires backend.', vim.log.levels.WARN)
    vim.notify('Try: cargo build to enable backend features', vim.log.levels.INFO)
    return
  end
  
  local name = feature_name or vim.fn.input('Feature name: ')
  if name == '' then
    return
  end
  
  M.send_to_rust({
    type = 'new_spec',
    data = { feature_name = name }
  })
end

-- Open existing spec
function M.open_spec(spec_name)
  if not state.initialized then
    vim.notify('Agent backend not available. Spec opening requires backend.', vim.log.levels.WARN)
    vim.notify('Try: cargo build to enable backend features', vim.log.levels.INFO)
    return
  end
  
  local name = spec_name
  if not name then
    local specs = M.list_specs()
    if #specs == 0 then
      vim.notify('No specs found in .kiro/specs/', vim.log.levels.WARN)
      return
    end
    
    vim.ui.select(specs, {
      prompt = 'Select spec to open:',
      format_item = function(item)
        return item
      end,
    }, function(choice)
      if choice then
        M.send_to_rust({
          type = 'open_spec',
          data = { spec_name = choice }
        })
      end
    end)
  else
    M.send_to_rust({
      type = 'open_spec',
      data = { spec_name = name }
    })
  end
end

-- Close agent interface
function M.close_agent()
  M.send_to_rust({ type = 'close_agent' })
end

-- Send message to Rust backend
function M.send_to_rust(message)
  if not state.rust_job_id then
    return false
  end
  
  local ok, json_message = pcall(vim.json.encode, message)
  if not ok then
    return false
  end
  
  local success = pcall(vim.fn.chansend, state.rust_job_id, json_message .. '\n')
  if not success then
    return false
  end
  
  return true
end

-- Send message via gRPC
function M.send_message_grpc()
  local message = ui.get_input()
  
  if message == '' then
    return
  end
  
  ui.clear_input()
  
  if not state.chat_history then
    state.chat_history = {}
  end
  
  table.insert(state.chat_history, '**You:** ' .. message)
  table.insert(state.chat_history, '')
  
  -- Gather context
  local ctx = context.gather_all()
  local sanitized_ctx = context.sanitize(ctx)
  
  -- Send via gRPC
  communication.send_chat(message, sanitized_ctx, function(response)
    if response.success then
      table.insert(state.chat_history, '**Agent:** ' .. response.payload.message)
      table.insert(state.chat_history, '')
      ui.update_chat(state.chat_history)
    else
      table.insert(state.chat_history, '**System:** Error: ' .. (response.error or 'Unknown error'))
      table.insert(state.chat_history, '')
      ui.update_chat(state.chat_history)
    end
  end)
  
  ui.update_chat(state.chat_history)
  
  if ui.is_open() then
    vim.schedule(function()
      vim.cmd('startinsert')
    end)
  end
end

-- Handle window creation from Rust
function M.handle_window_create(data)
  ui.create_interface()
end

-- Handle window updates from Rust
function M.handle_window_update(data)
  if data.content then
    ui.update_chat(data.content)
  end
end

-- Handle chat response from Rust
function M.handle_chat_response(data)
  if not state.chat_history then
    state.chat_history = {}
  end
  
  table.insert(state.chat_history, data.message)
  table.insert(state.chat_history, '')
  
  ui.update_chat(state.chat_history)
end

-- Handle spec updates from Rust
function M.handle_spec_update(data)
  state.current_spec = data.spec_name
  vim.notify('Spec updated: ' .. data.action, vim.log.levels.INFO)
end

-- List available specs
function M.list_specs()
  local spec_dir = vim.fn.getcwd() .. '/.kiro/specs'
  if vim.fn.isdirectory(spec_dir) == 0 then
    return {}
  end
  
  local specs = {}
  local handle = vim.loop.fs_scandir(spec_dir)
  if handle then
    while true do
      local name, type = vim.loop.fs_scandir_next(handle)
      if not name then break end
      if type == 'directory' then
        table.insert(specs, name)
      end
    end
  end
  
  return specs
end

-- Show plugin status
function M.show_status()
  local grpc_status = communication.is_connected() and 'Connected' or 'Disconnected'
  local comm_state = communication.get_state()
  
  local status = {
    'nvim-spec-agent Status:',
    '  Backend: ' .. (state.initialized and 'Running' or 'Stopped'),
    '  Job ID: ' .. (state.rust_job_id or 'None'),
    '  gRPC: ' .. grpc_status,
    '  Pending Requests: ' .. comm_state.pending_requests,
    '  Current Spec: ' .. (state.current_spec or 'None'),
    '  UI State: ' .. (ui.is_open() and 'Open' or 'Closed'),
  }
  
  vim.notify(table.concat(status, '\n'), vim.log.levels.INFO)
end

-- Get plugin configuration
function M.get_config()
  return config
end

-- Get plugin state
function M.get_state()
  return state
end

-- Debug function
function M.debug_paths()
  local plugin_dir = get_plugin_dir()
  local paths = {
    plugin_dir .. '/bin/nvim-spec-agent',
    plugin_dir .. '/bin/nvim-spec-agent.exe',
    plugin_dir .. '/target/release/nvim-spec-agent',
    plugin_dir .. '/target/release/nvim-spec-agent.exe',
  }
  
  vim.notify('Plugin directory: ' .. plugin_dir, vim.log.levels.INFO)
  vim.notify('Searching for binary in:', vim.log.levels.INFO)
  for _, path in ipairs(paths) do
    local exists = vim.fn.filereadable(path) == 1
    vim.notify('  ' .. path .. ' - ' .. (exists and 'EXISTS' or 'NOT FOUND'), vim.log.levels.INFO)
  end
  
  vim.notify('Build script: ' .. plugin_dir .. '/build.sh - ' .. 
    (vim.fn.filereadable(plugin_dir .. '/build.sh') == 1 and 'EXISTS' or 'NOT FOUND'), vim.log.levels.INFO)
end

-- Initialize plugin
function M.init()
  if not config then
    M.setup({})
  end
end

return M
