-- Communication Module
-- Provides gRPC client and protocol handling for agent communication
local M = {}

-- Load submodules
M.grpc_client = require('agent.communication.grpc_client')
M.protocol = require('agent.communication.protocol')

-- Initialize communication system
function M.init(endpoint)
  M.grpc_client.init(endpoint)
end

-- Send a chat message
function M.send_chat(message, context, callback)
  local request = M.protocol.create_chat_request(message, context)
  return M.grpc_client.send_request('chat', request, callback)
end

-- Send a spec operation
function M.send_spec_operation(operation, data, callback)
  local request = M.protocol.create_spec_request(operation, data)
  return M.grpc_client.send_request('spec_operation', request, callback)
end

-- Send a command execution request
function M.send_command(command, working_dir, context, callback)
  local request = M.protocol.create_command_request(command, working_dir, context)
  return M.grpc_client.send_request('command_execution', request, callback)
end

-- Send context update
function M.send_context(context_data, callback)
  local request = M.protocol.create_context_message(context_data)
  return M.grpc_client.send_request('context_update', request, callback)
end

-- Health check
function M.health_check(callback)
  local request = M.protocol.create_health_check()
  return M.grpc_client.send_request('health_check', request, callback)
end

-- Check if connected
function M.is_connected()
  return M.grpc_client.is_connected()
end

-- Disconnect
function M.disconnect()
  M.grpc_client.disconnect()
end

-- Get communication state
function M.get_state()
  return M.grpc_client.get_state()
end

return M
