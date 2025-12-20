-- Agent.nvim Lua Interface
local M = {}

-- Plugin state
local state = {
  initialized = false,
  rust_job_id = nil,
  windows = {},
  current_spec = nil,
  chat_history = {},
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
    open_agent = '<leader>af',
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

-- Find the prebuilt binary for current platform
local function find_rust_binary()
  if config.rust_binary_path then
    return config.rust_binary_path
  end
  
  -- Get the plugin root directory
  local plugin_dir = get_plugin_dir()
  
  -- Try to use the platform-specific binary selector
  local selector_path = plugin_dir .. '/bin/select-binary.lua'
  if vim.fn.filereadable(selector_path) == 1 then
    local selector = dofile(selector_path)
    local binary_path = selector.get_binary_path(plugin_dir)
    if binary_path then
      vim.notify('Found platform-specific binary: ' .. binary_path, vim.log.levels.DEBUG)
      return binary_path
    end
  end
  
  -- Fallback: try generic binary names (including debug builds)
  local fallback_paths = {
    plugin_dir .. '/bin/nvim-spec-agent',
    plugin_dir .. '/bin/nvim-spec-agent.exe',
    plugin_dir .. '/target/debug/nvim-spec-agent',     -- Debug build
    plugin_dir .. '/target/debug/nvim-spec-agent.exe', -- Debug build Windows
    plugin_dir .. '/target/release/nvim-spec-agent',   -- Release build
    plugin_dir .. '/target/release/nvim-spec-agent.exe', -- Release build Windows
    'nvim-spec-agent', -- In PATH
  }
  
  for _, path in ipairs(fallback_paths) do
    if vim.fn.executable(path) == 1 then
      vim.notify('Found fallback binary: ' .. path, vim.log.levels.DEBUG)
      return path
    end
  end
  
  return nil
end

-- Setup function
function M.setup(user_config)
  config = vim.tbl_deep_extend('force', default_config, user_config or {})
  
  -- Find the Rust binary (but don't fail setup if not found)
  config.rust_binary_path = find_rust_binary()
  
  if not config.rust_binary_path then
    local plugin_dir = get_plugin_dir()
    vim.notify('agent.nvim: Rust binary not found initially', vim.log.levels.WARN)
    vim.notify('Plugin directory: ' .. plugin_dir, vim.log.levels.INFO)
    vim.notify('Binary will be searched again when needed', vim.log.levels.INFO)
    
    -- Check if Rust is available
    if vim.fn.executable('cargo') == 1 then
      vim.notify('Cargo found. You can build with: cd ' .. plugin_dir .. ' && cargo build', vim.log.levels.INFO)
    else
      vim.notify('Cargo not found. Install Rust from: https://rustup.rs/', vim.log.levels.WARN)
    end
  end
  
  -- ALWAYS set up keybindings, regardless of binary status
  vim.schedule(function()
    if config.keybindings.open_agent then
      vim.keymap.set('n', config.keybindings.open_agent, M.toggle_agent, { 
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
  
  -- Auto-start if configured and binary is available
  if config.auto_start and config.rust_binary_path then
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
  elseif msg_type == 'chat_response' then
    M.handle_chat_response(message.data)
  elseif msg_type == 'notification' then
    vim.notify(message.data.text, message.data.level)
  elseif msg_type == 'spec_update' then
    M.handle_spec_update(message.data)
  end
end

-- Toggle agent interface (open/close)
function M.toggle_agent()
  -- Check if interface is already open (check both input and chat windows)
  local is_open = false
  
  if state.windows.input and vim.api.nvim_win_is_valid(state.windows.input.win) then
    is_open = true
  elseif state.windows.chat and vim.api.nvim_win_is_valid(state.windows.chat.win) then
    is_open = true
  end
  
  if is_open then
    -- If open, close the interface
    M.close_agent_interface()
    return
  end
  
  -- If not open, open it
  M.open_agent()
end

-- Open agent interface (always opens, doesn't toggle)
function M.open_agent()
  -- Re-check for binary if not found during setup
  if not config.rust_binary_path then
    config.rust_binary_path = find_rust_binary()
    if not config.rust_binary_path then
      vim.notify('agent.nvim: Rust binary still not found', vim.log.levels.ERROR)
      vim.notify('Try building with: cargo build', vim.log.levels.INFO)
      return
    end
  end
  
  -- Ensure backend is running
  if not state.initialized then
    vim.notify('Starting agent backend...', vim.log.levels.INFO)
    if not M.start_rust_backend() then
      vim.notify('Failed to start agent backend', vim.log.levels.ERROR)
      return
    end
    
    -- Wait a moment for backend to initialize
    vim.defer_fn(function()
      M.create_dual_window_interface()
    end, 200)
  else
    -- Create the dual window interface directly
    M.create_dual_window_interface()
  end
  
  -- Also notify Rust backend
  if state.initialized then
    M.send_to_rust({ type = 'open_agent' })
  end
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
    vim.notify('Rust backend not running', vim.log.levels.WARN)
    return false
  end
  
  local ok, json_message = pcall(vim.json.encode, message)
  if not ok then
    vim.notify('Failed to encode message: ' .. tostring(json_message), vim.log.levels.ERROR)
    return false
  end
  
  local success = pcall(vim.fn.chansend, state.rust_job_id, json_message .. '\n')
  if not success then
    vim.notify('Failed to send message to backend', vim.log.levels.ERROR)
    return false
  end
  
  return true
end

-- Create dual window interface (chat history + input)
function M.create_dual_window_interface()
  -- Ensure we have valid dimensions
  if vim.o.columns < 20 or vim.o.lines < 10 then
    vim.notify('Terminal too small for agent interface', vim.log.levels.WARN)
    return
  end
  
  -- Calculate dimensions
  local width = math.floor(vim.o.columns * (config.ui.window_width_ratio or 0.8))
  local total_height = math.floor(vim.o.lines * (config.ui.window_height_ratio or 0.6))
  local input_height = 3
  local chat_height = total_height - input_height - 1 -- -1 for spacing
  
  -- Ensure minimum dimensions
  width = math.max(width, 40)
  chat_height = math.max(chat_height, 5)
  
  -- Calculate positions (centered)
  local col = math.floor((vim.o.columns - width) / 2)
  local chat_row = math.floor((vim.o.lines - total_height) / 2)
  local input_row = chat_row + chat_height + 1 -- +1 for spacing
  
  -- Create chat history window (only if there's content)
  if state.chat_history and #state.chat_history > 0 then
    -- Create chat buffer if it doesn't exist
    if not state.windows.chat or not vim.api.nvim_buf_is_valid(state.windows.chat.buf) then
      local chat_buf = vim.api.nvim_create_buf(false, true)
      
      -- Set buffer options safely
      pcall(vim.api.nvim_buf_set_option, chat_buf, 'filetype', 'markdown')
      pcall(vim.api.nvim_buf_set_option, chat_buf, 'wrap', true)
      pcall(vim.api.nvim_buf_set_option, chat_buf, 'conceallevel', 2)
      pcall(vim.api.nvim_buf_set_option, chat_buf, 'concealcursor', 'nv')
      
      -- Set chat history content
      vim.api.nvim_buf_set_lines(chat_buf, 0, -1, false, state.chat_history)
      
      local chat_config = {
        relative = 'editor',
        width = width,
        height = chat_height,
        col = col,
        row = chat_row,
        style = 'minimal',
        border = config.ui.border_style,
        title = 'Agent Chat History',
        title_pos = 'center',
        zindex = 40,
      }
      
      local ok, chat_win = pcall(vim.api.nvim_open_win, chat_buf, false, chat_config)
      if not ok then
        vim.notify('Failed to create chat window: ' .. tostring(chat_win), vim.log.levels.ERROR)
        return
      end
      
      -- Enable syntax highlighting in the window
      pcall(vim.api.nvim_win_call, chat_win, function()
        vim.cmd('syntax enable')
        if vim.fn.exists('syntax_on') == 0 then
          vim.cmd('syntax on')
        end
      end)
      
      state.windows.chat = {
        buf = chat_buf,
        win = chat_win,
        config = chat_config,
      }
      
      -- Set up keymaps for chat window
      vim.keymap.set('n', 'q', function()
        M.close_agent_interface()
      end, { buffer = chat_buf, noremap = true, silent = true })
    end
  end
  
  -- Create input window
  if not state.windows.input or not vim.api.nvim_buf_is_valid(state.windows.input.buf) then
    local input_buf = vim.api.nvim_create_buf(false, true)
    pcall(vim.api.nvim_buf_set_option, input_buf, 'filetype', 'markdown')
    
    local input_config = {
      relative = 'editor',
      width = width,
      height = input_height,
      col = col,
      row = input_row,
      style = 'minimal',
      border = config.ui.border_style,
      title = 'Message Input',
      title_pos = 'center',
      zindex = 50, -- Higher z-index for input window
    }
    
    local ok, input_win = pcall(vim.api.nvim_open_win, input_buf, true, input_config)
    if not ok then
      vim.notify('Failed to create input window: ' .. tostring(input_win), vim.log.levels.ERROR)
      return
    end
    
    state.windows.input = {
      buf = input_buf,
      win = input_win,
      config = input_config,
    }
    
    -- Set up keymaps for input window
    vim.keymap.set('n', '<CR>', function()
      M.send_message()
    end, { buffer = input_buf, noremap = true, silent = true })
    
    vim.keymap.set('i', '<C-CR>', function()
      M.send_message()
    end, { buffer = input_buf, noremap = true, silent = true })
    
    vim.keymap.set('n', '<Esc>', function()
      M.close_agent_interface()
    end, { buffer = input_buf, noremap = true, silent = true })
    
    vim.keymap.set('i', '<Esc>', function()
      M.close_agent_interface()
    end, { buffer = input_buf, noremap = true, silent = true })
    
    -- Start in insert mode for immediate typing
    vim.schedule(function()
      if vim.api.nvim_win_is_valid(input_win) then
        vim.api.nvim_set_current_win(input_win)
        vim.cmd('startinsert')
      end
    end)
  end
end

-- Close agent interface
function M.close_agent_interface()
  local closed_windows = 0
  
  -- Close chat window
  if state.windows.chat then
    if vim.api.nvim_win_is_valid(state.windows.chat.win) then
      pcall(vim.api.nvim_win_close, state.windows.chat.win, true)
      closed_windows = closed_windows + 1
    end
    state.windows.chat = nil
  end
  
  -- Close input window
  if state.windows.input then
    if vim.api.nvim_win_is_valid(state.windows.input.win) then
      pcall(vim.api.nvim_win_close, state.windows.input.win, true)
      closed_windows = closed_windows + 1
    end
    state.windows.input = nil
  end
  
  -- Clear the entire windows table to ensure clean state
  state.windows = {}
  
  if closed_windows > 0 then
    vim.notify('Agent interface closed', vim.log.levels.INFO)
  end
  
  -- Also notify Rust backend
  if state.initialized then
    M.send_to_rust({ type = 'close_agent' })
  end
end

-- Send message from input window
function M.send_message()
  if not state.windows.input or not vim.api.nvim_buf_is_valid(state.windows.input.buf) then
    return
  end
  
  -- Get message from input buffer
  local lines = vim.api.nvim_buf_get_lines(state.windows.input.buf, 0, -1, false)
  local message = table.concat(lines, '\n'):gsub('^%s*(.-)%s*$', '%1') -- trim whitespace
  
  if message == '' then
    return
  end
  
  -- Clear input buffer
  vim.api.nvim_buf_set_lines(state.windows.input.buf, 0, -1, false, { '' })
  
  -- Add message to chat history
  if not state.chat_history then
    state.chat_history = {}
  end
  
  table.insert(state.chat_history, '**You:** ' .. message)
  table.insert(state.chat_history, '')
  
  -- Send to Rust backend
  M.send_to_rust({
    type = 'chat_message',
    data = { message = message }
  })
  
  -- Update chat window if it exists
  if state.windows.chat and vim.api.nvim_buf_is_valid(state.windows.chat.buf) then
    vim.api.nvim_buf_set_lines(state.windows.chat.buf, 0, -1, false, state.chat_history)
    -- Scroll to bottom
    local line_count = #state.chat_history
    vim.api.nvim_win_set_cursor(state.windows.chat.win, {line_count, 0})
  else
    -- Create chat window if it doesn't exist but we now have history
    M.create_dual_window_interface()
  end
  
  -- Keep focus on input window
  if state.windows.input and vim.api.nvim_win_is_valid(state.windows.input.win) then
    vim.api.nvim_set_current_win(state.windows.input.win)
    vim.cmd('startinsert')
  end
end

-- Handle window creation from Rust (updated)
function M.handle_window_create(data)
  -- Use our dual window system instead of single window
  M.create_dual_window_interface()
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

-- Handle chat response from Rust
function M.handle_chat_response(data)
  if not state.chat_history then
    state.chat_history = {}
  end
  
  -- Add agent response to chat history
  table.insert(state.chat_history, data.message)
  table.insert(state.chat_history, '')
  
  -- Update chat window if it exists
  if state.windows.chat and vim.api.nvim_buf_is_valid(state.windows.chat.buf) then
    vim.api.nvim_buf_set_lines(state.windows.chat.buf, 0, -1, false, state.chat_history)
    -- Scroll to bottom
    local line_count = #state.chat_history
    vim.api.nvim_win_set_cursor(state.windows.chat.win, {line_count, 0})
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

-- Initialize plugin with default configuration
function M.init()
  if not config then
    M.setup({})
  end
end

return M