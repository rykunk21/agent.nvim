-- Auto-select the correct binary for the current platform
local M = {}

function M.get_binary_path(plugin_dir)
  local os_name = vim.loop.os_uname().sysname:lower()
  local arch = vim.loop.os_uname().machine:lower()
  
  -- Normalize architecture
  if arch == "x86_64" or arch == "amd64" then
    arch = "x64"
  elseif arch == "aarch64" or arch == "arm64" then
    arch = "arm64"
  end
  
  local binary_name
  if os_name:match("linux") then
    binary_name = "nvim-spec-agent-linux-" .. arch
  elseif os_name:match("darwin") then
    binary_name = "nvim-spec-agent-macos-" .. arch
  elseif os_name:match("windows") then
    binary_name = "nvim-spec-agent-windows-" .. arch .. ".exe"
  else
    return nil
  end
  
  local binary_path = plugin_dir .. "/bin/" .. binary_name
  return vim.fn.filereadable(binary_path) == 1 and binary_path or nil
end

return M
