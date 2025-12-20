-- gRPC Client for Lua Frontend
-- Handles communication with Rust controller via gRPC
local M = {}

-- Client state
local client_state = {
  connected = false,
  job_id = nil,
  endpoint = 'localhost:50051',
  message_queue = {},
  pending_requests = {},
  request_counter = 0,
  callbacks = {},
  health_check_interval = 5000, -- 5 seconds
  last_health_check = 0,
  reconnect_attempts = 0,
  max_reconnect_attempts = 5,
  reconnect_delay = 1000, -- 1 second
}

-- Message types for gRPC communication
local MESSAGE_TYPES = {
  CHAT = 'chat',
  SPEC_OPERATION = 'spec_operation',
  COMMAND_EXECUTION = 'command_execution',
  FILE_OPERATION = 'file_operation',
  HEALTH_CHECK = 'health_check',
  CONTEXT_UPDATE = 'context_update',
}

-- Response types
local RESPONSE_TYPES = {
  SUCCESS = 'success',
  ERROR = 'error',
  STREAMING = 'streaming',
  HEALTH_OK = 'health_ok',
}

-- Initialize gRPC client
function M.init(endpoint)
  if endpoint then
    client_state.endpoint = endpoint
  end
  
  -- Start the gRPC communication process
  M.connect()
end

-- Connect to Rust controller
function M.connect()
  if client_state.connected then
    return true
  end
  
  -- Start a background process that handles gRPC communication
  -- For now, we'll use a simple TCP connection approach
  local cmd = {
    'nc', -- netcat for TCP communication
    '-l', -- listen mode (will be replaced with actual gRPC client)
    client_state.endpoint
  }
  
  -- In a real implementation, this would use a proper gRPC client library
  -- For now, we'll use a placeholder that can be replaced with grpc-lua or similar
  
  client_state.connected = true
  client_state.reconnect_attempts = 0
  
  return true
end

-- Disconnect from Rust controller
function M.disconnect()
  if client_state.job_id then
    vim.fn.jobstop(client_state.job_id)
    client_state.job_id = nil
  end
  
  client_state.connected = false
end

-- Send a request to the Rust controller
-- Returns a request ID that can be used to track the response
function M.send_request(request_type, payload, callback)
  if not client_state.connected then
    if callback then
      callback({
        success = false,
        error = 'Not connected to controller',
      })
    end
    return nil
  end
  
  -- Generate unique request ID
  client_state.request_counter = client_state.request_counter + 1
  local request_id = 'req_' .. client_state.request_counter
  
  -- Create gRPC message
  local message = {
    id = request_id,
    request_type = request_type,
    payload = payload,
    timestamp = vim.fn.localtime(),
  }
  
  -- Store callback for later response handling
  if callback then
    client_state.callbacks[request_id] = callback
  end
  
  -- Queue the message
  table.insert(client_state.message_queue, message)
  
  -- Send the message
  M.flush_queue()
  
  return request_id
end

-- Send a chat message
function M.send_chat(message, callback)
  return M.send_request(MESSAGE_TYPES.CHAT, {
    message = message,
  }, callback)
end

-- Send a spec operation request
function M.send_spec_operation(operation, data, callback)
  return M.send_request(MESSAGE_TYPES.SPEC_OPERATION, {
    operation = operation,
    data = data,
  }, callback)
end

-- Send a command execution request
function M.send_command(command, working_dir, callback)
  return M.send_request(MESSAGE_TYPES.COMMAND_EXECUTION, {
    command = command,
    working_directory = working_dir,
  }, callback)
end

-- Send file operation request
function M.send_file_operation(operation, file_path, content, callback)
  return M.send_request(MESSAGE_TYPES.FILE_OPERATION, {
    operation = operation,
    file_path = file_path,
    content = content,
  }, callback)
end

-- Send context update
function M.send_context(context_data, callback)
  return M.send_request(MESSAGE_TYPES.CONTEXT_UPDATE, context_data, callback)
end

-- Health check
function M.health_check(callback)
  return M.send_request(MESSAGE_TYPES.HEALTH_CHECK, {}, callback)
end

-- Flush message queue - send all pending messages
function M.flush_queue()
  if not client_state.connected or #client_state.message_queue == 0 then
    return
  end
  
  -- In a real implementation, this would serialize messages to gRPC format
  -- and send them over the network
  for _, message in ipairs(client_state.message_queue) do
    M.send_message_internal(message)
  end
  
  -- Clear the queue
  client_state.message_queue = {}
end

-- Internal function to send a single message
function M.send_message_internal(message)
  -- Serialize to JSON for now (would be gRPC protobuf in production)
  local ok, json_str = pcall(vim.json.encode, message)
  if not ok then
    return false
  end
  
  -- In a real implementation, this would send over gRPC
  -- For now, we'll use a placeholder
  
  return true
end

-- Handle response from Rust controller
function M.handle_response(response)
  if not response or not response.id then
    return
  end
  
  -- Find and execute the callback
  local callback = client_state.callbacks[response.id]
  if callback then
    callback(response)
    client_state.callbacks[response.id] = nil
  end
end

-- Handle streaming response
function M.handle_streaming_response(response)
  if not response or not response.id then
    return
  end
  
  local callback = client_state.callbacks[response.id]
  if callback then
    callback(response)
    
    -- Keep callback for streaming responses
    if response.type ~= RESPONSE_TYPES.STREAMING then
      client_state.callbacks[response.id] = nil
    end
  end
end

-- Periodic health check
function M.periodic_health_check()
  local now = vim.fn.localtime() * 1000 -- Convert to milliseconds
  
  if now - client_state.last_health_check >= client_state.health_check_interval then
    client_state.last_health_check = now
    
    M.health_check(function(response)
      if response.success then
        client_state.reconnect_attempts = 0
      else
        M.handle_connection_error()
      end
    end)
  end
end

-- Handle connection error
function M.handle_connection_error()
  client_state.connected = false
  client_state.reconnect_attempts = client_state.reconnect_attempts + 1
  
  if client_state.reconnect_attempts <= client_state.max_reconnect_attempts then
    -- Schedule reconnection
    vim.defer_fn(function()
      M.connect()
    end, client_state.reconnect_delay * client_state.reconnect_attempts)
  else
    vim.notify('Failed to reconnect to gRPC controller after ' .. 
      client_state.max_reconnect_attempts .. ' attempts', vim.log.levels.ERROR)
  end
end

-- Get connection status
function M.is_connected()
  return client_state.connected
end

-- Get pending request count
function M.get_pending_request_count()
  return vim.tbl_count(client_state.pending_requests)
end

-- Cancel a pending request
function M.cancel_request(request_id)
  if client_state.callbacks[request_id] then
    client_state.callbacks[request_id] = nil
    return true
  end
  return false
end

-- Set custom endpoint
function M.set_endpoint(endpoint)
  client_state.endpoint = endpoint
end

-- Get current endpoint
function M.get_endpoint()
  return client_state.endpoint
end

-- Get client state (for debugging)
function M.get_state()
  return {
    connected = client_state.connected,
    endpoint = client_state.endpoint,
    pending_requests = vim.tbl_count(client_state.pending_requests),
    message_queue_size = #client_state.message_queue,
    reconnect_attempts = client_state.reconnect_attempts,
  }
end

return M
