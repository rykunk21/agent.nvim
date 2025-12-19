-- Download prebuilt binary for agent.nvim
local M = {}

function M.get_platform()
  local os_name = vim.loop.os_uname().sysname:lower()
  local arch = vim.loop.os_uname().machine:lower()
  
  -- Normalize architecture names
  if arch == "x86_64" or arch == "amd64" then
    arch = "x86_64"
  elseif arch == "aarch64" or arch == "arm64" then
    arch = "aarch64"
  end
  
  -- Determine platform
  if os_name:match("linux") then
    return arch .. "-unknown-linux-gnu"
  elseif os_name:match("darwin") then
    return arch .. "-apple-darwin"
  elseif os_name:match("windows") then
    return arch .. "-pc-windows-msvc"
  else
    return nil
  end
end

function M.get_binary_name()
  local os_name = vim.loop.os_uname().sysname:lower()
  return os_name:match("windows") and "nvim-spec-agent.exe" or "nvim-spec-agent"
end

function M.download_binary(plugin_dir)
  local platform = M.get_platform()
  if not platform then
    vim.notify("Unsupported platform, falling back to build from source", vim.log.levels.WARN)
    return false
  end
  
  local binary_name = M.get_binary_name()
  local version = "latest" -- or get from git tag
  local url = string.format(
    "https://github.com/rykunk21/agent.nvim/releases/%s/download/%s",
    version,
    binary_name
  )
  
  vim.notify("Downloading prebuilt binary for " .. platform .. "...", vim.log.levels.INFO)
  
  -- Create bin directory
  vim.fn.system("mkdir -p " .. plugin_dir .. "/bin")
  
  -- Download binary
  local download_cmd = string.format(
    "curl -L -o %s/bin/%s %s",
    plugin_dir,
    binary_name,
    url
  )
  
  local result = vim.fn.system(download_cmd)
  if vim.v.shell_error == 0 then
    -- Make executable on Unix
    if not vim.loop.os_uname().sysname:lower():match("windows") then
      vim.fn.system("chmod +x " .. plugin_dir .. "/bin/" .. binary_name)
    end
    vim.notify("Binary downloaded successfully!", vim.log.levels.INFO)
    return true
  else
    vim.notify("Download failed: " .. result, vim.log.levels.ERROR)
    return false
  end
end

return M