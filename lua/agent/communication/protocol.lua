-- gRPC Protocol Handler
-- Handles message serialization and deserialization for gRPC communication
local M = {}

-- Message format version
local PROTOCOL_VERSION = '1.0'

-- Message envelope structure
local function create_message_envelope(message_type, payload, request_id)
  return {
    version = PROTOCOL_VERSION,
    id = request_id or vim.fn.system('uuidgen'):gsub('\n', ''),
    type = message_type,
    timestamp = os.time(),
    payload = payload,
  }
end

-- Serialize message to JSON
function M.serialize(message)
  local ok, json_str = pcall(vim.json.encode, message)
  if not ok then
    return nil, 'Failed to serialize message: ' .. tostring(json_str)
  end
  return json_str
end

-- Deserialize message from JSON
function M.deserialize(json_str)
  local ok, message = pcall(vim.json.decode, json_str)
  if not ok then
    return nil, 'Failed to deserialize message: ' .. tostring(message)
  end
  return message
end

-- Create a chat request message
function M.create_chat_request(message, context)
  return create_message_envelope('chat_request', {
    message = message,
    context = context,
  })
end

-- Create a spec operation request
function M.create_spec_request(operation, data)
  return create_message_envelope('spec_request', {
    operation = operation,
    data = data,
  })
end

-- Create a command execution request
function M.create_command_request(command, working_dir, context)
  return create_message_envelope('command_request', {
    command = command,
    working_directory = working_dir,
    context = context,
  })
end

-- Create a context update message
function M.create_context_message(context_data)
  return create_message_envelope('context_update', context_data)
end

-- Create a health check message
function M.create_health_check()
  return create_message_envelope('health_check', {})
end

-- Parse response message
function M.parse_response(response_json)
  local response = M.deserialize(response_json)
  if not response then
    return nil
  end
  
  -- Validate response structure
  if not response.id or not response.type then
    return nil
  end
  
  return response
end

-- Validate message structure
function M.validate_message(message)
  if not message then
    return false, 'Message is nil'
  end
  
  if not message.version then
    return false, 'Missing protocol version'
  end
  
  if not message.id then
    return false, 'Missing message ID'
  end
  
  if not message.type then
    return false, 'Missing message type'
  end
  
  return true
end

-- Create error response
function M.create_error_response(request_id, error_message)
  return {
    version = PROTOCOL_VERSION,
    id = request_id,
    type = 'error_response',
    timestamp = os.time(),
    error = error_message,
  }
end

-- Create success response
function M.create_success_response(request_id, payload)
  return {
    version = PROTOCOL_VERSION,
    id = request_id,
    type = 'success_response',
    timestamp = os.time(),
    payload = payload,
  }
end

-- Create streaming response
function M.create_streaming_response(request_id, chunk, is_final)
  return {
    version = PROTOCOL_VERSION,
    id = request_id,
    type = 'streaming_response',
    timestamp = os.time(),
    chunk = chunk,
    is_final = is_final or false,
  }
end

return M
