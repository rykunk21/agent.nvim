-- Agent.nvim Lua Interface
local M = {}

-- Plugin state
local state = {
  initialized = false,
  rust_job_id = nil,
  windows = {},
  current_spec = nil,
}

-- Configuration
local default_config = {
  auto_start = false,
  rust_binary_path = nil, -- Will be auto-detected
  log_level = 'info',
  ui = {
    border_style = 'rounded',
    window_width_ratio = 0.8,
    window_height_ratio = 0.6,
  },
  keybindings = {
    open_agent = '<leader>sa',
    new_spec = '<leader>sn',
    open_spec = '<leader>so',
    close_agent = '<Esc>',
  }
}

local config = default_config

-- Get the plugin directory
local function get_plugin_dir()
  -- Get the directory of this script file
  local str = debug.getinfo(1, "S").source:sub(2)
  -- Get the directory containing this file
  local script_dir = vim.fn.fnamemodify(str, ':h')
  -- Navigate from lua/agent/ back to plugin root (two levels up)
  return vim.fn.fnamemodify(script_dir, ':h:h')
end

-- Find the Rust binary
local function find_rust_binary()
  if config.rust_binary_path then
    return config.rust_binary_path
  end
  
  -- Get the plugin root directory
  local plugin_dir = get_plugin_dir()
  
  -- Try different possible locations
  local possible_paths = {
    plugin_dir .. '/bin/nvim-spec-agent',
    plugin_dir .. '/bin/nvim-spec-agent.exe',
    plugin_dir .. '/target/release/nvim-spec-agent',
    plugin_dir .. '/target/release/nvim-spec-agent.exe',
    'nvim-spec-agent', -- In PATH
  }
  
  for _, path in ipairs(possible_paths) do
    if vim.fn.executable(path) == 1 then
      vim.notify('Found Rust binary at: ' .. path, vim.log.levels.DEBUG)
      return path
    end
  end
  
  vim.notify('Rust binary not found. Searched paths: ' .. vim.inspect(possible_paths), vim.log.levels.WARN)
  return nil
end

-- Setup function
function M.setup(user_config)
  config = vim.tbl_deep_extend('force', default_config, user_config or {})
  
  -- Find the Rust binary
  config.rust_binary_path = find_rust_binary()
  
  if not config.rust_binary_path then
    local plugin_dir = get_plugin_dir()
    vim.notify('agent.nvim: Rust binary not found!', vim.log.levels.ERROR)
    vim.notify('Plugin directory: ' .. plugin_dir, vim.log.levels.INFO)
    
    -- Check if build script exists
    local build_script = plugin_dir .. '/build.sh'
    if vim.fn.filereadable(build_script) == 1 then
      vim.notify('Build script found. Try running manually:', vim.log.levels.INFO)
      vim.notify('  cd ' .. plugin_dir .. ' && ./build.sh', vim.log.levels.INFO)
    else
      vim.notify('Build script not found at: ' .. build_script, vim.log.levels.WARN)
    end
    
    -- Check if Rust is available
    if vim.fn.executable('cargo') == 1 then
      vim.notify('Cargo found. You can build manually:', vim.log.levels.INFO)
      vim.notify('  cd ' .. plugin_dir .. ' && cargo build --release', vim.log.levels.INFO)
    else
      vim.notify('Cargo not found. Install Rust from: https://rustup.rs/', vim.log.levels.WARN)
    end
    
    return
  end
  
  -- Set up keybindings
  if config.keybindings.open_agent then
    vim.keymap.set('n', config.keybindings.open_agent, M.open_agent, { desc = 'Open Spec Agent' })
  end
  
  if config.keybindings.new_spec then
    vim.keymap.set('n', config.keybindings.new_spec, M.new_spec, { desc = 'New Spec' })
  end
  
  if config.keybindings.open_spec then
    vim.keymap.set('n', config.keybindings.open_spec, M.open_spec, { desc = 'Open Spec' })
  end
  
  -- Auto-start if configured
  if config.auto_start then
    M.start_rust_backend()
  end
end

-- Start the Rust backend
function M.start_rust_backend()
  if state.rust_job_id then
    return true -- Already running
  end
  
  if not config.rust_binary_path then
    vim.notify('agent.nvim: Rust binary not found', vim.log.levels.ERROR)
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
    vim.notify('Failed to start agent.nvim backend', vim.log.levels.ERROR)
    return false
  end
  
  state.initialized = true
  vim.notify('agent.nvim backend started', vim.log.levels.INFO)
  
  -- Send a ping to test the connection
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
      -- Parse JSON messages from Rust backend
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
      vim.notify('Rust backend error: ' .. line, vim.log.levels.ERROR)
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
  elseif msg_type == 'notification' then
    vim.notify(message.data.text, message.data.level)
  elseif msg_type == 'spec_update' then
    M.handle_spec_update(message.data)
  end
end

-- Open agent interface
function M.open_agent()
  if not state.initialized then
    if not M.start_rust_backend() then
      return
    end
  end
  
  -- Send message to Rust backend to open agent
  M.send_to_rust({ type = 'open_agent' })
end

-- Create new spec
function M.new_spec(feature_name)
  if not state.initialized then
    vim.notify('Agent backend not initialized', vim.log.levels.ERROR)
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
    vim.notify('Agent backend not initialized', vim.log.levels.ERROR)
    return
  end
  
  local name = spec_name
  if not name or name == '' then
    -- Show spec selection UI
    local specs = M.list_specs()
    if #specs == 0 then
      vim.notify('No specs found', vim.log.levels.INFO)
      return
    end
    
    vim.ui.select(specs, {
      prompt = 'Select spec:',
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
    vim.notify('Rust backend not running', vim.log.levels.ERROR)
    return
  end
  
  local json_message = vim.json.encode(message)
  vim.fn.chansend(state.rust_job_id, json_message .. '\n')
end

-- Handle window creation from Rust
function M.handle_window_create(data)
  -- Implementation for creating windows based on Rust backend requests
  local buf = vim.api.nvim_create_buf(false, true)
  
  local win_config = {
    relative = 'editor',
    width = data.width,
    height = data.height,
    col = data.col,
    row = data.row,
    style = 'minimal',
    border = config.ui.border_style,
  }
  
  local win = vim.api.nvim_open_win(buf, data.focusable or false, win_config)
  
  state.windows[data.window_type] = {
    buf = buf,
    win = win,
    config = win_config,
  }
  
  -- Set buffer content if provided
  if data.content then
    vim.api.nvim_buf_set_lines(buf, 0, -1, false, data.content)
  end
end

-- Handle window updates from Rust
function M.handle_window_update(data)
  local window = state.windows[data.window_type]
  if not window then
    return
  end
  
  if data.content then
    vim.api.nvim_buf_set_lines(window.buf, 0, -1, false, data.content)
  end
  
  if data.cursor then
    vim.api.nvim_win_set_cursor(window.win, data.cursor)
  end
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

-- Auto-start function
function M.auto_start()
  if config.auto_start then
    M.start_rust_backend()
  end
end

-- Save state on exit
function M.save_state()
  M.send_to_rust({ type = 'save_state' })
end

-- Handle window resize
function M.handle_resize()
  M.send_to_rust({ type = 'handle_resize' })
end

-- Show plugin status
function M.show_status()
  local status = {
    'nvim-spec-agent Status:',
    '  Backend: ' .. (state.initialized and 'Running' or 'Stopped'),
    '  Job ID: ' .. (state.rust_job_id or 'None'),
    '  Current Spec: ' .. (state.current_spec or 'None'),
    '  Windows: ' .. vim.tbl_count(state.windows),
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

-- Debug function to check plugin directory detection
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

return M