-- Context Provider
-- Gathers comprehensive context from Neovim for the agent
local M = {}

-- Gather all available context
function M.gather_all_context()
  return {
    buffer = M.gather_buffer_context(),
    files = M.gather_file_context(),
    diagnostics = M.gather_diagnostic_context(),
    project = M.gather_project_context(),
    edit_history = M.gather_edit_history(),
  }
end

-- Gather current buffer context
function M.gather_buffer_context()
  local current_buf = vim.api.nvim_get_current_buf()
  
  if not vim.api.nvim_buf_is_valid(current_buf) then
    return nil
  end
  
  local lines = vim.api.nvim_buf_get_lines(current_buf, 0, -1, false)
  local cursor = vim.api.nvim_win_get_cursor(0)
  local name = vim.api.nvim_buf_get_name(current_buf)
  local filetype = vim.api.nvim_buf_get_option(current_buf, 'filetype')
  local modified = vim.api.nvim_buf_get_option(current_buf, 'modified')
  
  return {
    path = name,
    content = table.concat(lines, '\n'),
    cursor_line = cursor[1],
    cursor_column = cursor[2],
    filetype = filetype,
    modified = modified,
    line_count = #lines,
  }
end

-- Gather file system context
function M.gather_file_context()
  local cwd = vim.fn.getcwd()
  local open_buffers = {}
  
  -- Get all open buffers
  for _, buf in ipairs(vim.api.nvim_list_bufs()) do
    if vim.api.nvim_buf_is_valid(buf) and vim.api.nvim_buf_is_loaded(buf) then
      local name = vim.api.nvim_buf_get_name(buf)
      if name ~= '' then
        table.insert(open_buffers, {
          path = name,
          filetype = vim.api.nvim_buf_get_option(buf, 'filetype'),
          modified = vim.api.nvim_buf_get_option(buf, 'modified'),
        })
      end
    end
  end
  
  return {
    working_directory = cwd,
    open_buffers = open_buffers,
    project_structure = M.gather_project_structure(cwd),
  }
end

-- Gather project structure
function M.gather_project_structure(root_dir)
  local structure = {}
  
  -- Scan root directory for key files and folders
  local scan_depth = 2
  local function scan_dir(dir, depth)
    if depth > scan_depth then
      return {}
    end
    
    local items = {}
    local handle = vim.loop.fs_scandir(dir)
    
    if handle then
      while true do
        local name, type = vim.loop.fs_scandir_next(handle)
        if not name then break end
        
        -- Skip hidden files and common ignore patterns
        if not name:match('^%.') and 
           name ~= 'node_modules' and 
           name ~= '.git' and 
           name ~= 'target' and
           name ~= '__pycache__' then
          
          local full_path = dir .. '/' .. name
          table.insert(items, {
            name = name,
            type = type,
            path = full_path,
          })
          
          -- Recursively scan subdirectories
          if type == 'directory' and depth < scan_depth then
            local sub_items = scan_dir(full_path, depth + 1)
            if #sub_items > 0 then
              items[#items].children = sub_items
            end
          end
        end
      end
    end
    
    return items
  end
  
  return scan_dir(root_dir, 0)
end

-- Gather diagnostic context
function M.gather_diagnostic_context()
  local diagnostics = {}
  
  -- Get diagnostics for current buffer
  local current_buf = vim.api.nvim_get_current_buf()
  local buf_diagnostics = vim.diagnostic.get(current_buf)
  
  for _, diag in ipairs(buf_diagnostics) do
    table.insert(diagnostics, {
      line = diag.lnum,
      column = diag.col,
      severity = diag.severity,
      message = diag.message,
      source = diag.source,
    })
  end
  
  return {
    current_buffer_diagnostics = diagnostics,
    diagnostic_count = #diagnostics,
  }
end

-- Gather project context
function M.gather_project_context()
  local cwd = vim.fn.getcwd()
  
  -- Check for common project files
  local project_files = {
    'package.json',
    'Cargo.toml',
    'pyproject.toml',
    'go.mod',
    'pom.xml',
    'build.gradle',
    'Gemfile',
    'composer.json',
  }
  
  local detected_projects = {}
  for _, file in ipairs(project_files) do
    if vim.fn.filereadable(cwd .. '/' .. file) == 1 then
      table.insert(detected_projects, file)
    end
  end
  
  return {
    root_directory = cwd,
    detected_project_files = detected_projects,
  }
end

-- Gather edit history
function M.gather_edit_history()
  -- Get undo history information
  local undo_info = {}
  
  -- Try to get recent changes from undo tree
  if vim.fn.exists('*undotree') == 1 then
    local tree = vim.fn.undotree()
    if tree and tree.entries then
      -- Get last 10 entries
      local entries = tree.entries
      local start_idx = math.max(1, #entries - 9)
      
      for i = start_idx, #entries do
        local entry = entries[i]
        table.insert(undo_info, {
          time = entry.time,
          number = entry.number,
        })
      end
    end
  end
  
  return {
    recent_changes = undo_info,
    change_count = #undo_info,
  }
end

-- Sanitize context for transmission
function M.sanitize_context(context)
  -- Remove sensitive information
  local sanitized = vim.deepcopy(context)
  
  -- Truncate large content
  if sanitized.buffer and sanitized.buffer.content then
    local max_size = 100000 -- 100KB limit
    if #sanitized.buffer.content > max_size then
      sanitized.buffer.content = sanitized.buffer.content:sub(1, max_size) .. '\n... (truncated)'
    end
  end
  
  -- Remove paths that might contain sensitive info
  if sanitized.files and sanitized.files.working_directory then
    -- Keep only the last part of the path for privacy
    local path = sanitized.files.working_directory
    local parts = vim.split(path, '/')
    sanitized.files.working_directory = parts[#parts] or path
  end
  
  return sanitized
end

-- Get context for a specific file
function M.get_file_context(file_path)
  local buf = nil
  
  -- Find buffer for file
  for _, b in ipairs(vim.api.nvim_list_bufs()) do
    if vim.api.nvim_buf_is_valid(b) then
      local name = vim.api.nvim_buf_get_name(b)
      if name == file_path then
        buf = b
        break
      end
    end
  end
  
  if not buf then
    return nil
  end
  
  local lines = vim.api.nvim_buf_get_lines(buf, 0, -1, false)
  
  return {
    path = file_path,
    content = table.concat(lines, '\n'),
    filetype = vim.api.nvim_buf_get_option(buf, 'filetype'),
    modified = vim.api.nvim_buf_get_option(buf, 'modified'),
  }
end

-- Get context for a range of lines
function M.get_line_range_context(start_line, end_line)
  local current_buf = vim.api.nvim_get_current_buf()
  local lines = vim.api.nvim_buf_get_lines(current_buf, start_line - 1, end_line, false)
  
  return {
    start_line = start_line,
    end_line = end_line,
    content = table.concat(lines, '\n'),
    line_count = #lines,
  }
end

-- Get context around cursor
function M.get_cursor_context(lines_before, lines_after)
  lines_before = lines_before or 5
  lines_after = lines_after or 5
  
  local current_buf = vim.api.nvim_get_current_buf()
  local cursor = vim.api.nvim_win_get_cursor(0)
  local cursor_line = cursor[1]
  
  local start_line = math.max(1, cursor_line - lines_before)
  local end_line = math.min(vim.api.nvim_buf_line_count(current_buf), cursor_line + lines_after)
  
  return M.get_line_range_context(start_line, end_line)
end

-- Get LSP context if available
function M.get_lsp_context()
  local clients = vim.lsp.get_active_clients()
  local context = {
    active_clients = {},
  }
  
  for _, client in ipairs(clients) do
    table.insert(context.active_clients, {
      name = client.name,
      id = client.id,
    })
  end
  
  return context
end

return M
