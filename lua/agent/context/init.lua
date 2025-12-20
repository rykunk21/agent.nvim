-- Context Module
-- Provides context gathering and management for the agent
local M = {}

-- Load submodules
M.provider = require('agent.context.provider')

-- Gather all context
function M.gather_all()
  return M.provider.gather_all_context()
end

-- Gather specific context types
function M.gather_buffer()
  return M.provider.gather_buffer_context()
end

function M.gather_files()
  return M.provider.gather_file_context()
end

function M.gather_diagnostics()
  return M.provider.gather_diagnostic_context()
end

function M.gather_project()
  return M.provider.gather_project_context()
end

function M.gather_edit_history()
  return M.provider.gather_edit_history()
end

-- Get context for specific file
function M.get_file_context(file_path)
  return M.provider.get_file_context(file_path)
end

-- Get context around cursor
function M.get_cursor_context(lines_before, lines_after)
  return M.provider.get_cursor_context(lines_before, lines_after)
end

-- Get LSP context
function M.get_lsp_context()
  return M.provider.get_lsp_context()
end

-- Sanitize context for transmission
function M.sanitize(context)
  return M.provider.sanitize_context(context)
end

return M
