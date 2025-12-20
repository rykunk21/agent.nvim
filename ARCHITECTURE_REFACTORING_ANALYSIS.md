# Architecture Refactoring Analysis and Roadmap

## Executive Summary

This document provides a comprehensive analysis of the existing nvim-spec-agent codebase and outlines the refactoring roadmap to migrate from the current two-layer architecture (Lua frontend + Rust backend) to a three-layer architecture (Lua frontend + Rust controller + Docker container with MCP orchestration layer).

## Current Architecture Analysis

### 1. Existing Rust Backend Assessment

#### Current Structure
```
src/
├── main.rs                    # JSON protocol communication entry point
├── lib.rs                     # Main plugin coordination (currently unused in binary)
├── agent/                     # Chat and conversation management
├── ui/                        # Window management and visual components
├── spec/                      # Spec-driven development workflow
├── config/                    # Configuration and persistence
├── utils/                     # Error handling and Neovim API wrappers
├── communication/             # JSON protocol for Lua-Rust communication
└── examples/                  # Demo implementations
```

#### Components Analysis

**✅ Reusable Components (Can be adapted):**
- `communication/` - JSON protocol foundation can be extended to gRPC
- `config/` - Configuration management patterns applicable to container configs
- `utils/error_handling.rs` - Error types and handling patterns
- `spec/workflow.rs` - Core spec workflow logic (can be moved to container)

**🔄 Requires Significant Refactoring:**
- `main.rs` - Currently JSON-based, needs gRPC server/client implementation
- `lib.rs` - Plugin coordination needs to become container management
- `agent/` - Chat management needs to move to MCP orchestration layer
- `ui/` - Window management stays but communication changes to gRPC

**❌ Requires Complete Rewrite:**
- Direct Neovim integration patterns (moving to Lua-only)
- Synchronous communication patterns (moving to async gRPC)
- Single-process architecture assumptions

#### Current Communication Pattern
```
Neovim Lua ←→ JSON over stdin/stdout ←→ Rust Binary
```

**Issues with Current Pattern:**
- Synchronous communication limits responsiveness
- No support for streaming operations
- Limited error recovery and connection management
- No support for external service integration (MCP)

### 2. Existing Lua Frontend Assessment

#### Current Structure
```
lua/agent/init.lua             # Main plugin interface and state management
plugin/agent.vim               # Plugin registration and commands
autoload/health/agent.vim      # Health check functionality
```

#### Components Analysis

**✅ Reusable Components:**
- Window creation and management logic
- Plugin registration and command setup
- Health check framework
- Configuration management patterns
- Binary detection and path resolution

**🔄 Requires Significant Refactoring:**
- Communication layer (JSON → gRPC)
- State management (local → distributed across layers)
- Error handling (sync → async patterns)
- Context gathering (needs comprehensive implementation)

**❌ Missing Components (Need Implementation):**
- gRPC client implementation
- Comprehensive context gathering system
- Real-time UI updates from streaming responses
- Container lifecycle awareness

#### Current Communication Pattern
```lua
-- Current: Direct JSON communication
vim.fn.chansend(job_id, json_message .. '\n')

-- Future: gRPC client communication
grpc_client:send_request(request_proto)
```

### 3. Missing Components for Three-Layer Architecture

#### Container Infrastructure
- Docker container specification
- Container lifecycle management
- Health monitoring and recovery
- Resource management and cleanup

#### MCP Orchestration Layer
- LLM provider abstraction (Ollama, OpenAI, Anthropic)
- MCP client for external services
- gRPC server for controller communication
- Conversation and context management

#### gRPC Communication
- Protocol buffer definitions
- Bidirectional streaming support
- Connection health monitoring
- Message correlation and async handling

## Target Architecture Design

### 1. Three-Layer Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    Neovim + Lua Frontend                     │
│  Context Gathering │ gRPC Client │ Window Management │ UI   │
├─────────────────────────────────────────────────────────────┤
│                    Rust Controller Binary                    │
│ Container Manager │ gRPC Server/Client │ Process Management │
├─────────────────────────────────────────────────────────────┤
│                   Docker Container                          │
│  MCP Orchestration │ LLM Providers │ gRPC Server │ Spec Eng │
└─────────────────────────────────────────────────────────────┘
```

### 2. Container Architecture Specification

#### Docker Container Design

**Base Image:** `python:3.11-slim`
**Key Components:**
- MCP orchestration engine (Python)
- gRPC server for controller communication
- LLM provider integrations
- MCP client for external services
- Spec-driven development engine

**Container Structure:**
```
container/
├── Dockerfile
├── requirements.txt
├── src/
│   ├── main.py                    # Container entry point
│   ├── orchestration/
│   │   ├── llm_manager.py         # LLM provider abstraction
│   │   ├── mcp_engine.py          # MCP orchestration
│   │   └── spec_engine.py         # Spec workflow engine
│   ├── providers/
│   │   ├── ollama.py              # Local Ollama integration
│   │   ├── openai.py              # OpenAI API integration
│   │   └── anthropic.py           # Anthropic API integration
│   ├── communication/
│   │   ├── grpc_server.py         # gRPC server implementation
│   │   └── protocol.py            # Message protocol
│   └── mcp/
│       ├── client.py              # MCP client for external services
│       └── services.py            # MCP service management
└── config/
    ├── llm_providers.json         # LLM provider configurations
    └── mcp_services.json          # MCP service configurations
```

#### LLM Provider Abstraction Layer

**Unified Interface:**
```python
class LLMProvider:
    async def generate_response(self, messages: List[Message], context: Context) -> Response
    async def stream_response(self, messages: List[Message], context: Context) -> AsyncIterator[Response]
    def supports_streaming(self) -> bool
    def get_capabilities(self) -> ProviderCapabilities
```

**Provider Implementations:**
- **Ollama Provider:** Local model integration via HTTP API
- **OpenAI Provider:** Cloud API with secure key management
- **Anthropic Provider:** Cloud API with secure key management

#### MCP Client Integration

**External Service Connections:**
- Outbound-only connections (container is not an MCP server)
- Authentication and authorization handling
- Service discovery and endpoint management
- Graceful degradation when services unavailable

### 3. gRPC Communication Design

#### Protocol Buffer Definitions
```protobuf
service AgentService {
  rpc SendMessage(MessageRequest) returns (MessageResponse);
  rpc StreamConversation(stream ConversationRequest) returns (stream ConversationResponse);
  rpc ManageSpec(SpecRequest) returns (SpecResponse);
  rpc ExecuteCommand(CommandRequest) returns (CommandResponse);
  rpc HealthCheck(HealthRequest) returns (HealthResponse);
}

message MessageRequest {
  string id = 1;
  string content = 2;
  Context context = 3;
  MessageType type = 4;
}

message Context {
  BufferInfo current_buffer = 1;
  repeated FileInfo open_files = 2;
  ProjectStructure project = 3;
  repeated DiagnosticInfo diagnostics = 4;
}
```

#### Communication Patterns
- **Request/Response:** Standard operations (spec creation, command execution)
- **Bidirectional Streaming:** Real-time conversation and file operations
- **Server Streaming:** Long-running operations with progress updates
- **Client Streaming:** Large context data transmission

## Migration Roadmap

### Phase 1: Foundation (Tasks 2-3)
1. **Rust Controller Refactoring**
   - Implement Docker container management
   - Create gRPC server for Lua communication
   - Create gRPC client for container communication
   - Migrate configuration management

2. **Lua Frontend Refactoring**
   - Implement gRPC client
   - Add comprehensive context gathering
   - Update window management for async operations
   - Maintain backward compatibility during transition

### Phase 2: Container Implementation (Task 4)
1. **MCP Orchestration Layer**
   - Create Docker container with Python-based orchestration
   - Implement LLM provider manager
   - Create MCP client for external services
   - Implement gRPC server for controller communication

### Phase 3: Feature Migration (Tasks 5-6)
1. **Spec-Driven Development Migration**
   - Port existing spec workflow to container
   - Update command execution through container
   - Migrate file operation monitoring

2. **Enhanced Context System**
   - Implement comprehensive context gathering
   - Add intelligent context filtering
   - Optimize context transmission

### Phase 4: Distribution and Optimization (Tasks 7-8)
1. **Prebuilt Binary System**
   - Update build system for container integration
   - Create CI/CD pipeline for multi-platform builds
   - Update plugin manager integration

2. **Performance and Reliability**
   - Optimize gRPC communication
   - Implement comprehensive error handling
   - Add performance monitoring

## Risk Assessment and Mitigation

### High-Risk Areas
1. **Communication Protocol Migration**
   - Risk: Breaking existing functionality during JSON → gRPC transition
   - Mitigation: Implement dual protocol support during transition

2. **Container Dependency**
   - Risk: Docker not available on user systems
   - Mitigation: Graceful degradation, clear error messages, fallback modes

3. **Performance Regression**
   - Risk: Three-layer architecture may introduce latency
   - Mitigation: Async patterns, connection pooling, local caching

### Medium-Risk Areas
1. **Configuration Complexity**
   - Risk: More complex setup for users
   - Mitigation: Sensible defaults, auto-configuration, clear documentation

2. **External Service Dependencies**
   - Risk: MCP services may be unreliable
   - Mitigation: Graceful degradation, retry logic, offline modes

## Success Criteria

### Technical Metrics
- Container startup time < 5 seconds
- gRPC communication latency < 100ms for typical operations
- Memory usage increase < 200MB compared to current implementation
- All existing functionality preserved

### User Experience Metrics
- Installation process remains simple (single plugin manager command)
- No breaking changes to existing keybindings and commands
- Improved responsiveness for long-running operations
- Enhanced agent capabilities through MCP integration

## Implementation Guidelines

### Development Principles
1. **Backward Compatibility:** Maintain existing API during transition
2. **Graceful Degradation:** Plugin should work even if container fails
3. **Clear Error Messages:** Help users troubleshoot configuration issues
4. **Performance First:** Optimize for responsiveness and resource usage
5. **Security:** Secure API key management and container isolation

### Testing Strategy
1. **Unit Tests:** Each component tested in isolation
2. **Integration Tests:** End-to-end workflow testing
3. **Container Tests:** Docker container functionality
4. **Performance Tests:** Latency and resource usage benchmarks
5. **Compatibility Tests:** Multiple Neovim versions and plugin managers

This analysis provides the foundation for implementing the three-layer architecture while maintaining the plugin's usability and extending its capabilities through MCP integration.