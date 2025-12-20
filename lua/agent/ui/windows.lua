-- Window Management
-- Manages floating windows for the agent interface with gRPC communication
local M = {}

-- Window state
local window_state = {
  chat_window = nil,
  input_window = nil,
  command_window = nil,
  progress_window = nil,
  windows_open = false,
}

-- Configuration
local config = {
  border_style = 'rounded',
  window_width_ratio = 0.8,
  window_height_ratio = 0.6,
  chat_height_ratio = 0.7,
  input_height = 3,
}

-- Set configuration
function M.set_config(cfg)
  config = vim.tbl_deep_extend('force', config, cfg or {})
end

-- Create dual window interface (chat + input)
function M.create_dual_window()
  if window_state.windows_open then
    return true
  end
  
  -- Validate terminal size
  if vim.o.columns < 40 or vim.o.lines < 15 then
    vim.notify('Terminal too small for agent interface', vim.log.levels.WARN)
    return false
  end
  
  -- Calculate dimensions
  local width = math.floor(vim.o.columns * config.window_width_ratio)
  local total_height = math.floor(vim.o.lines * config.window_height_ratio)
  local chat_height = math.floor(total_height * config.chat_height_ratio)
  local input_height = config.input_height
  
  -- Ensure minimum dimensions
  width = math.max(width, 40)
  chat_height = math.max(chat_height, 5)
  
  -- Calculate positions (centered)
  local col = math.floor((vim.o.columns - width) / 2)
  local chat_row = math.floor((vim.o.lines - total_height) / 2)
  local input_row = chat_row + chat_height + 1
  
  -- Create chat window
  local chat_buf = vim.api.nvim_create_buf(false, true)
  vim.api.nvim_buf_set_option(chat_buf, 'filetype', 'markdown')
  vim.api.nvim_buf_set_option(chat_buf, 'wrap', true)
  
  local chat_config = {
    relative = 'editor',
    width = width,
    height = chat_height,
    col = col,
    row = chat_row,
    style = 'minimal',
    border = config.border_style,
    title = 'Agent Chat History',
    title_pos = 'center',
    zindex = 40,
  }
  
  local ok, chat_win = pcall(vim.api.nvim_open_win, chat_buf, false, chat_config)
  if not ok then
    vim.notify('Failed to create chat window', vim.log.levels.ERROR)
    return false
  end
  
  window_state.chat_window = {
    buf = chat_buf,
    win = chat_win,
    config = chat_config,
  }
  
  -- Set up chat window keymaps
  M.setup_chat_keymaps(chat_buf)
  
  -- Create input window
  local input_buf = vim.api.nvim_create_buf(false, true)
  vim.api.nvim_buf_set_option(input_buf, 'filetype', 'markdown')
  
  local input_config = {
    relative = 'editor',
    width = width,
    height = input_height,
    col = col,
    row = input_row,
    style = 'minimal',
    border = config.border_style,
    title = 'Message Input',
    title_pos = 'center',
    zindex = 50,
  }
  
  local ok, input_win = pcall(vim.api.nvim_open_win, input_buf, true, input_config)
  if not ok then
    vim.notify('Failed to create input window', vim.log.levels.ERROR)
    vim.api.nvim_win_close(chat_win, true)
    return false
  end
  
  window_state.input_window = {
    buf = input_buf,
    win = input_win,
    config = input_config,
  }
  
  -- Set up input window keymaps
  M.setup_input_keymaps(input_buf)
  
  -- Start in insert mode
  vim.schedule(function()
    if vim.api.nvim_win_is_valid(input_win) then
      vim.api.nvim_set_current_win(input_win)
      vim.cmd('startinsert')
    end
  end)
  
  window_state.windows_open = true
  return true
end

-- Close all windows
function M.close_all_windows()
  if window_state.chat_window then
    if vim.api.nvim_win_is_valid(window_state.chat_window.win) then
      pcall(vim.api.nvim_win_close, window_state.chat_window.win, true)
    end
    window_state.chat_window = nil
  end
  
  if window_state.input_window then
    if vim.api.nvim_win_is_valid(window_state.input_window.win) then
      pcall(vim.api.nvim_win_close, window_state.input_window.win, true)
    end
    window_state.input_window = nil
  end
  
  if window_state.command_window then
    if vim.api.nvim_win_is_valid(window_state.command_window.win) then
      pcall(vim.api.nvim_win_close, window_state.command_window.win, true)
    end
    window_state.command_window = nil
  end
  
  if window_state.progress_window then
    if vim.api.nvim_win_is_valid(window_state.progress_window.win) then
      pcall(vim.api.nvim_win_close, window_state.progress_window.win, true)
    end
    window_state.progress_window = nil
  end
  
  window_state.windows_open = false
end

-- Update chat window content
function M.update_chat_content(lines)
  if not window_state.chat_window or not vim.api.nvim_buf_is_valid(window_state.chat_window.buf) then
    return false
  end
  
  vim.api.nvim_buf_set_lines(window_state.chat_window.buf, 0, -1, false, lines)
  
  -- Scroll to bottom
  if vim.api.nvim_win_is_valid(window_state.chat_window.win) then
    local line_count = #lines
    vim.api.nvim_win_set_cursor(window_state.chat_window.win, {line_count, 0})
  end
  
  return true
end

-- Get input window content
function M.get_input_content()
  if not window_state.input_window or not vim.api.nvim_buf_is_valid(window_state.input_window.buf) then
    return nil
  end
  
  local lines = vim.api.nvim_buf_get_lines(window_state.input_window.buf, 0, -1, false)
  return table.concat(lines, '\n'):gsub('^%s*(.-)%s*$', '%1')
end

-- Clear input window
function M.clear_input()
  if not window_state.input_window or not vim.api.nvim_buf_is_valid(window_state.input_window.buf) then
    return false
  end
  
  vim.api.nvim_buf_set_lines(window_state.input_window.buf, 0, -1, false, { '' })
  return true
end

-- Show progress indicator
function M.show_progress(message)
  if not window_state.progress_window or not vim.api.buf_is_valid(window_state.progress_window.buf) then
    -- Create progress window
    local buf = vim.api.nvim_create_buf(false, true)
    
    local config = {
      relative = 'editor',
      width = 40,
      height = 3,
      col = math.floor((vim.o.columns - 40) / 2),
      row = math.floor((vim.o.lines - 3) / 2),
      style = 'minimal',
      border = 'rounded',
      title = 'Processing',
      zindex = 60,
    }
    
    local ok, win = pcall(vim.api.nvim_open_win, buf, false, config)
    if ok then
      window_state.progress_window = {
        buf = buf,
        win = win,
        config = config,
      }
    end
  end
  
  if window_state.progress_window and vim.api.nvim_buf_is_valid(window_state.progress_window.buf) then
    vim.api.nvim_buf_set_lines(window_state.progress_window.buf, 0, -1, false, {
      '',
      '  ' .. message .. ' ...',
      '',
    })
  end
end

-- Hide progress indicator
function M.hide_progress()
  if window_state.progress_window then
    if vim.api.nvim_win_is_valid(window_state.progress_window.win) then
      pcall(vim.api.nvim_win_close, window_state.progress_window.win, true)
    end
    window_state.progress_window = nil
  end
end

-- Show command approval window
function M.show_command_approval(command, description, on_approve, on_reject)
  local width = 60
  local height = 10
  
  local buf = vim.api.nvim_create_buf(false, true)
  
  local content = {
    '═══════════════════════════════════════════════════════════',
    'Command Approval Required',
    '═══════════════════════════════════════════════════════════',
    '',
    'Description: ' .. description,
    '',
    'Command:',
    '  ' .. command,
    '',
    'Press [y] to approve or [n] to reject',
  }
  
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, content)
  
  local config = {
    relative = 'editor',
    width = width,
    height = height,
    col = math.floor((vim.o.columns - width) / 2),
    row = math.floor((vim.o.lines - height) / 2),
    style = 'minimal',
    border = 'double',
    title = 'Approval',
    zindex = 70,
  }
  
  local ok, win = pcall(vim.api.nvim_open_win, buf, true, config)
  if not ok then
    return false
  end
  
  window_state.command_window = {
    buf = buf,
    win = win,
    config = config,
  }
  
  -- Set up keymaps for approval
  vim.keymap.set('n', 'y', function()
    M.close_command_approval()
    if on_approve then on_approve() end
  end, { buffer = buf, noremap = true, silent = true })
  
  vim.keymap.set('n', 'n', function()
    M.close_command_approval()
    if on_reject then on_reject() end
  end, { buffer = buf, noremap = true, silent = true })
  
  vim.keymap.set('n', '<Esc>', function()
    M.close_command_approval()
    if on_reject then on_reject() end
  end, { buffer = buf, noremap = true, silent = true })
  
  return true
end

-- Close command approval window
function M.close_command_approval()
  if window_state.command_window then
    if vim.api.nvim_win_is_valid(window_state.command_window.win) then
      pcall(vim.api.nvim_win_close, window_state.command_window.win, true)
    end
    window_state.command_window = nil
  end
end

-- Handle window resize
function M.handle_resize()
  if window_state.windows_open then
    M.close_all_windows()
    M.create_dual_window()
  end
end

-- Check if windows are open
function M.are_windows_open()
  return window_state.windows_open
end

-- Get window state
function M.get_state()
  return {
    windows_open = window_state.windows_open,
    chat_window_valid = window_state.chat_window and vim.api.nvim_win_is_valid(window_state.chat_window.win),
    input_window_valid = window_state.input_window and vim.api.nvim_win_is_valid(window_state.input_window.win),
  }
end

-- Setup chat window keymaps
function M.setup_chat_keymaps(buf)
  vim.keymap.set('n', 'q', function()
    M.close_all_windows()
  end, { buffer = buf, noremap = true, silent = true })
  
  vim.keymap.set('n', '<Esc>', function()
    M.close_all_windows()
  end, { buffer = buf, noremap = true, silent = true })
end

-- Setup input window keymaps
function M.setup_input_keymaps(buf)
  vim.keymap.set('n', '<CR>', function()
    -- Will be handled by the main module
  end, { buffer = buf, noremap = true, silent = true })
  
  vim.keymap.set('i', '<C-CR>', function()
    -- Will be handled by the main module
  end, { buffer = buf, noremap = true, silent = true })
  
  vim.keymap.set('n', '<Esc>', function()
    M.close_all_windows()
  end, { buffer = buf, noremap = true, silent = true })
  
  vim.keymap.set('i', '<Esc>', function()
    M.close_all_windows()
  end, { buffer = buf, noremap = true, silent = true })
end

return M
