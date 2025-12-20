-- UI Module
-- Manages the user interface for the agent
local M = {}

-- Load submodules
M.windows = require('agent.ui.windows')

-- Initialize UI
function M.init(config)
  if config then
    M.windows.set_config(config)
  end
end

-- Create the dual window interface
function M.create_interface()
  return M.windows.create_dual_window()
end

-- Close all windows
function M.close_interface()
  M.windows.close_all_windows()
end

-- Update chat content
function M.update_chat(lines)
  return M.windows.update_chat_content(lines)
end

-- Get input content
function M.get_input()
  return M.windows.get_input_content()
end

-- Clear input
function M.clear_input()
  return M.windows.clear_input()
end

-- Show progress
function M.show_progress(message)
  M.windows.show_progress(message)
end

-- Hide progress
function M.hide_progress()
  M.windows.hide_progress()
end

-- Show command approval
function M.show_command_approval(command, description, on_approve, on_reject)
  return M.windows.show_command_approval(command, description, on_approve, on_reject)
end

-- Close command approval
function M.close_command_approval()
  M.windows.close_command_approval()
end

-- Handle resize
function M.handle_resize()
  M.windows.handle_resize()
end

-- Check if interface is open
function M.is_open()
  return M.windows.are_windows_open()
end

-- Get UI state
function M.get_state()
  return M.windows.get_state()
end

return M
