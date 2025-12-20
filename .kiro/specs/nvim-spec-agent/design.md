# Design Document: Enhanced Neovim Spec Agent Plugin

## Overview

The Enhanced Neovim Spec Agent Plugin is a three-layer architecture that provides an intelligent agent interface with spec-driven development capabilities. The plugin consists of a Lua frontend for Neovim integration, a Rust controller binary for container management, and a Docker container hosting a Rust-based MCP orchestration layer that can work with various LLM providers (Ollama, OpenAI, Anthropic). The plugin ships with prebuilt binaries and integrates seamlessly with Neovim's plugin ecosystem while providing advanced development automation features through gRPC communication between all layers.

## Architecture

### High-Level Architecture

The plugin follows a three-layer architecture with clear separation between the Neovim interface, container management, and the containerized agent:

```
┌─────────────────────────────────────────────────────────────┐
│                    Neovim + Lua Frontend                     │
│  UI Manager  │  Window Manager  │  Keymap Handler  │  gRPC   │
├─────────────────────────────────────────────────────────────┤
│                    Rust Controller Binary                    │
│ Container Mgr │ gRPC Server/Client │ Process Mgmt │ Config   │
├─────────────────────────────────────────────────────────────┤
│                   Docker Container                          │
│  MCP Orchestration Layer  │  LLM Providers  │  MCP Clients  │
│  (Ollama/OpenAI/Anthropic) │  gRPC Server   │  API Mgmt     │
└─────────────────────────────────────────────────────────────┘
```

### Communication Flow

```
Neovim Lua ←→ gRPC ←→ Rust Controller ←→ gRPC ←→ MCP Orchestration Layer
                                                      ↓
                                              LLM Providers (Ollama/OpenAI/Anthropic)
                                                      ↓
                                              Optional External MCP Services
```

### Component Interaction Flow

1. **User Input** → Lua Frontend → gRPC → Rust Controller → gRPC → MCP Orchestration Layer
2. **LLM Response** → gRPC → Rust Controller → gRPC → Lua Frontend → Neovim Display
3. **Container Management** → Rust Controller manages Docker lifecycle
4. **Context Gathering** → Neovim collects local context → Structured data to MCP Orchestration Layer
5. **LLM Provider Integration** → MCP Orchestration Layer routes to configured LLM (Ollama/OpenAI/Anthropic)
6. **MCP Integration** → MCP Orchestration Layer optionally connects outbound to external MCP services

## Components and Interfaces

### Neovim Lua Frontend

**UI Manager**
- Manages floating window creation and positioning
- Handles window resizing and z-index management
- Provides visual feedback for operations (read/write/command blocks)
- Implements responsive layout calculations
- Communicates with Rust controller via gRPC

**Window Manager**
- Creates and manages the two-window interface (chat + input)
- Handles window state persistence across sessions
- Manages window focus and navigation
- Implements adaptive sizing based on terminal dimensions

**Context Provider**
- Gathers current buffer contents, cursor position, and file metadata
- Collects file system paths, open files, and project structure
- Provides recent edits, undo/redo history, and change tracking
- Collects compiler/linter output, runtime errors, and stack traces
- Sanitizes and structures local context before sending to container

**gRPC Client**
- Establishes and maintains connection to Rust controller
- Handles request/response serialization and deserialization
- Implements connection health monitoring and retry logic
- Manages async communication patterns

### Rust Controller Binary

**Container Manager**
- Manages Docker container lifecycle (pull, start, stop, cleanup)
- Monitors container health and handles restart scenarios
- Configures container networking and resource limits
- Handles container image updates and version management

**gRPC Server/Client**
- Serves as gRPC server for Neovim Lua frontend
- Acts as gRPC client to containerized agent
- Routes messages between frontend and container
- Implements message correlation and async handling

**Process Manager**
- Manages the controller binary lifecycle
- Handles graceful shutdown and cleanup
- Monitors system resources and container status
- Implements error recovery and restart logic

**Configuration Manager**
- Manages plugin configuration and container settings
- Handles Docker image configuration and MCP service settings
- Provides configuration validation and migration
- Stores settings in standard Neovim configuration locations

### Docker Container (Rust-based MCP Orchestration Layer)

**LLM Provider Manager**
- Manages connections to different LLM providers (Ollama, OpenAI, Anthropic)
- Handles API key management and secure authentication for cloud providers
- Provides unified interface for LLM interactions regardless of provider
- Implements fallback mechanisms and error handling for provider failures

**MCP Orchestration Engine**
- Provides MCP capabilities to any configured LLM provider
- Routes LLM requests through MCP tools and external services
- Manages MCP protocol communication and tool discovery
- Handles MCP session management and state persistence

**gRPC Server**
- Serves requests from Rust controller
- Handles conversation management and context processing
- Implements streaming responses for long operations
- Provides health check and status endpoints

**MCP Client (Optional)**
- Connects outbound to external MCP-enabled services
- Handles authentication and permission management for external tools
- Provides graceful degradation when external services are unavailable
- Implements retry logic and error handling for MCP connections

**Spec Engine**
- Implements requirements, design, and task management
- Handles EARS-compliant requirement validation
- Manages correctness properties and testing strategies
- Tracks task completion and dependencies

## Data Models

### Core Data Structures

```rust
// gRPC Communication Messages
#[derive(Serialize, Deserialize)]
pub struct AgentRequest {
    pub id: String,
    pub request_type: RequestType,
    pub payload: serde_json::Value,
    pub context: Option<NeovimContext>,
}

#[derive(Serialize, Deserialize)]
pub struct AgentResponse {
    pub id: String,
    pub success: bool,
    pub payload: serde_json::Value,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub enum RequestType {
    Chat,
    SpecOperation,
    CommandExecution,
    FileOperation,
    HealthCheck,
}

// Neovim Context Data
#[derive(Serialize, Deserialize)]
pub struct NeovimContext {
    pub current_buffer: Option<BufferInfo>,
    pub cursor_position: CursorPosition,
    pub open_files: Vec<FileInfo>,
    pub project_structure: ProjectStructure,
    pub recent_edits: Vec<EditInfo>,
    pub diagnostics: Vec<DiagnosticInfo>,
}

#[derive(Serialize, Deserialize)]
pub struct BufferInfo {
    pub path: String,
    pub content: String,
    pub filetype: String,
    pub modified: bool,
}

// Container Management
#[derive(Serialize, Deserialize)]
pub struct ContainerConfig {
    pub image: String,
    pub tag: String,
    pub ports: Vec<PortMapping>,
    pub environment: HashMap<String, String>,
    pub llm_provider: LlmProviderConfig,
    pub mcp_services: Vec<McpServiceConfig>,
}

#[derive(Serialize, Deserialize)]
pub struct LlmProviderConfig {
    pub provider_type: LlmProviderType,
    pub config: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
pub enum LlmProviderType {
    Ollama {
        endpoint: String,
        model: String,
    },
    OpenAI {
        api_key: String,
        model: String,
        base_url: Option<String>,
    },
    Anthropic {
        api_key: String,
        model: String,
    },
}

#[derive(Serialize, Deserialize)]
pub struct McpServiceConfig {
    pub name: String,
    pub endpoint: String,
    pub auth: Option<AuthConfig>,
    pub enabled: bool,
}

// Conversation and session management
pub struct Conversation {
    pub id: String,
    pub messages: Vec<Message>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub spec_context: Option<SpecContext>,
}

pub struct Message {
    pub id: String,
    pub role: MessageRole,
    pub content: MessageContent,
    pub timestamp: DateTime<Utc>,
    pub metadata: MessageMetadata,
}

pub enum MessageContent {
    Text(String),
    OperationBlock(OperationBlock),
    CommandBlock(CommandBlock),
    SpecUpdate(SpecUpdate),
}

// Spec-driven development structures
pub struct SpecContext {
    pub feature_name: String,
    pub current_phase: SpecPhase,
    pub requirements: Option<RequirementsDocument>,
    pub design: Option<DesignDocument>,
    pub tasks: Option<TasksDocument>,
}

pub enum SpecPhase {
    Requirements,
    Design,
    Tasks,
    Implementation,
}

// Operation visualization
pub enum OperationBlock {
    ReadBlock {
        file_path: String,
        progress: f32,
        status: OperationStatus,
    },
    WriteBlock {
        file_path: String,
        content_preview: String,
        status: OperationStatus,
    },
}

// Command execution
pub struct CommandBlock {
    pub command: String,
    pub working_directory: String,
    pub description: String,
    pub risk_level: RiskLevel,
    pub approval_status: ApprovalStatus,
}

pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
    Executed { output: CommandOutput },
}
```

### Container State Management

```rust
pub struct ContainerState {
    pub container_id: Option<String>,
    pub status: ContainerStatus,
    pub health: HealthStatus,
    pub grpc_endpoint: Option<String>,
    pub last_health_check: DateTime<Utc>,
}

pub enum ContainerStatus {
    NotStarted,
    Starting,
    Running,
    Stopping,
    Stopped,
    Error(String),
}

pub struct HealthStatus {
    pub healthy: bool,
    pub last_response_time: Option<Duration>,
    pub error_count: u32,
    pub last_error: Option<String>,
}
```

### Window State Management

```rust
pub struct WindowState {
    pub chat_window: Option<WindowConfig>,
    pub input_window: Option<WindowConfig>,
    pub command_approval_window: Option<WindowConfig>,
    pub layout_mode: LayoutMode,
    pub dimensions: WindowDimensions,
}

pub struct WindowConfig {
    pub buffer_id: i32,
    pub window_id: i32,
    pub position: Position,
    pub size: Size,
    pub z_index: i32,
}

pub enum LayoutMode {
    Normal,
    CommandApproval,
    SpecNavigation,
}
```

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system-essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

Property 1: Plugin installation with prebuilt binaries
*For any* supported plugin manager and platform, installing the plugin should result in the correct prebuilt controller binary being present in the bin/ directory with execute permissions
**Validates: Requirements 1.1, 1.2, 1.4, 1.5**

Property 2: Plugin loading and registration
*For any* plugin load event, all required keybindings and commands should be registered in Neovim without requiring local compilation
**Validates: Requirements 1.3**

Property 3: Two-window interface creation
*For any* agent interface activation, the system should create exactly two properly positioned floating windows with the input window focused and correct z-index ordering
**Validates: Requirements 2.1, 2.2, 2.5**

Property 4: Responsive window layout
*For any* terminal resize event, all plugin windows should adjust their dimensions proportionally while maintaining proper positioning and readability
**Validates: Requirements 2.3**

Property 5: Conversation persistence
*For any* conversation session, closing and reopening the interface should preserve all conversation history and context
**Validates: Requirements 2.4**

Property 6: Spec workflow progression
*For any* new spec creation, the system should progress through requirements → design → tasks phases, creating properly formatted documents at each stage when approved
**Validates: Requirements 3.1, 3.2, 3.3**

Property 7: Spec navigation state preservation
*For any* navigation between spec phases, all document content and relationships should be preserved without data loss
**Validates: Requirements 3.4, 3.5**

Property 8: Operation visualization
*For any* file operation performed by the agent, the system should display appropriate read/write blocks with accurate progress indicators
**Validates: Requirements 4.1**

Property 9: Command approval workflow
*For any* command proposed by the agent, the system should present it with accept/reject options, execute only when approved, capture all output, and provide it to the agent
**Validates: Requirements 4.2, 4.4, 5.1, 5.2, 5.3, 5.4**

Property 10: Command rejection handling
*For any* command rejection, the system should cancel execution and return to normal chat mode without side effects
**Validates: Requirements 4.5**

Property 11: Dynamic layout adjustment
*For any* command block appearance, the chat window should shrink appropriately to accommodate the command interface
**Validates: Requirements 4.3**

Property 12: Error reporting completeness
*For any* command failure, the system should provide detailed error information to both user and agent including exit codes and error messages
**Validates: Requirements 5.5**

Property 13: Docker container lifecycle management
*For any* plugin startup, the system should check Docker availability, pull and start the containerized agent, establish gRPC communication, and handle failures with appropriate diagnostics
**Validates: Requirements 6.1, 6.2, 6.3, 6.4, 6.5**

Property 14: MCP service integration
*For any* configured external MCP service, the system should establish outbound connections, handle authentication, provide error reporting with retry mechanisms, and gracefully degrade when services are unavailable
**Validates: Requirements 7.1, 7.2, 7.3, 7.4, 7.5**

Property 15: gRPC communication chain integrity
*For any* request from the Lua frontend, the system should route it through the Rust controller to the containerized agent via gRPC, deliver responses back through the same chain, implement retry logic for errors, detect component failures, and maintain connection health monitoring
**Validates: Requirements 8.1, 8.2, 8.3, 8.4, 8.5**

Property 16: Enhanced agent capabilities with context provision
*For any* agent operation, the system should have Neovim gather and provide structured context (file contents, project structure, edit history, debugging information) to the containerized agent while maintaining workspace state understanding
**Validates: Requirements 9.1, 9.2, 9.3, 9.4, 9.5, 11.1, 11.2, 11.3, 11.4, 11.5**

Property 17: Concurrent operation safety and performance
*For any* set of concurrent operations, the system should handle them safely across the three-layer architecture without data corruption, maintain responsive UI during large operations, implement efficient memory management, and recover gracefully from crashes while preserving conversation history
**Validates: Requirements 10.1, 10.2, 10.3, 10.4, 10.5**

## Error Handling

### Error Categories and Strategies

**Plugin Installation Errors**
- Missing Docker or incompatible Docker versions
- Plugin manager compatibility issues
- File system permission problems for prebuilt binaries
- Network connectivity issues during container image download
- Platform-specific binary availability issues

**Container Management Errors**
- Docker daemon not running or inaccessible
- Container image pull failures due to network or registry issues
- Container startup failures due to resource constraints
- gRPC communication failures between controller and container
- Container health check failures and recovery scenarios

**Communication Errors**
- gRPC connection failures between Lua frontend and Rust controller
- Message serialization/deserialization errors
- Network timeouts and connection drops
- Message correlation failures in async operations
- Protocol version mismatches between components

**Context Gathering Errors**
- File system access errors when gathering project context
- Buffer content extraction failures
- Diagnostic information collection errors
- Large file handling and memory constraints
- Permission errors when accessing workspace files

**MCP Integration Errors**
- External MCP service connection failures
- Authentication and authorization errors for external services
- MCP protocol version incompatibilities
- Service discovery and endpoint resolution failures
- Graceful degradation when external services are unavailable

**Recovery Mechanisms**
- Automatic container restart with exponential backoff
- gRPC connection health monitoring and automatic reconnection
- Conversation history backup and restoration across component failures
- Graceful degradation when optional MCP services are unavailable
- User notification system for critical errors with recovery suggestions
- Diagnostic information collection for troubleshooting

## Testing Strategy

### Dual Testing Approach

The plugin will implement both unit testing and property-based testing to ensure comprehensive coverage across the three-layer architecture:

**Unit Testing**
- Specific examples of plugin installation across different managers and platforms
- Edge cases for window positioning with various terminal sizes
- Integration points between Neovim Lua frontend and Rust controller
- Container lifecycle management scenarios (start, stop, restart, failure)
- gRPC communication protocol edge cases and error scenarios
- Context gathering and serialization for different file types and project structures
- MCP service integration with various external service configurations

**Property-Based Testing**
- The plugin will use the `proptest` crate for Rust property-based testing and appropriate Lua testing frameworks
- Each property-based test will run a minimum of 100 iterations
- Each test will be tagged with comments referencing the design document properties
- Tag format: `**Feature: nvim-spec-agent, Property {number}: {property_text}**`
- Each correctness property will be implemented by a single property-based test

**Testing Framework Integration**
- Rust unit tests using the standard `#[test]` framework for controller binary
- Property-based tests using `proptest` with custom generators for Rust components
- Lua unit tests using `busted` or similar framework for Neovim integration
- Docker container tests using test containers for isolated container testing
- gRPC integration tests using test harnesses that mock different layers
- End-to-end tests that spawn complete three-layer architecture

### Test Coverage Requirements

**Core Functionality Tests**
- Plugin loading and initialization across different Neovim versions and plugin managers
- Container lifecycle management (pull, start, stop, health checks, recovery)
- gRPC communication chain integrity across all three layers
- Window management and layout calculations with responsive behavior
- Spec-driven development workflow state transitions
- Command execution and approval mechanisms with output capture
- File operation visualization and progress tracking

**Integration Tests**
- End-to-end spec creation and execution workflows across all layers
- Plugin manager installation verification with prebuilt binaries
- Cross-session conversation persistence with container restarts
- Error recovery and graceful degradation scenarios
- MCP service integration with external service mocking
- Context gathering and provision from Neovim to containerized agent

**Performance Tests**
- Memory usage monitoring during long conversations across all components
- Response time measurements for gRPC operations
- Concurrent command execution stress testing across the architecture
- Large file operation handling with container communication
- Container startup and shutdown performance benchmarks

**Container-Specific Tests**
- Docker image pull and container startup reliability
- Container health monitoring and automatic recovery
- gRPC server functionality within the container
- MCP client connectivity to external services from container
- Container resource usage and cleanup verification

## Implementation Architecture

### Three-Layer Plugin Structure

```
nvim-spec-agent/
├── Cargo.toml                    # Rust controller binary dependencies
├── Dockerfile                   # Container image definition for MCP-enabled LLM
├── docker-compose.yml           # Container orchestration configuration
├── bin/                         # Prebuilt controller binaries (from plugin-dist)
│   ├── nvim-spec-agent-linux-x64
│   ├── nvim-spec-agent-macos-x64
│   └── nvim-spec-agent-windows-x64.exe
├── src/                         # Rust controller binary source
│   ├── main.rs                  # Controller entry point and CLI interface
│   ├── container/
│   │   ├── mod.rs
│   │   ├── manager.rs           # Docker container lifecycle management
│   │   ├── health.rs            # Container health monitoring
│   │   └── config.rs            # Container configuration management
│   ├── communication/
│   │   ├── mod.rs
│   │   ├── grpc_server.rs       # gRPC server for Lua frontend
│   │   ├── grpc_client.rs       # gRPC client for container communication
│   │   ├── protocol.rs          # Message protocol definitions
│   │   └── types.rs             # Communication data types
│   ├── config/
│   │   ├── mod.rs
│   │   ├── settings.rs          # Plugin configuration management
│   │   └── persistence.rs       # Configuration persistence
│   └── utils/
│       ├── mod.rs
│       ├── error_handling.rs    # Error types and handling
│       └── logging.rs           # Logging and diagnostics
├── lua/                         # Neovim Lua frontend
│   └── agent/
│       ├── init.lua             # Main plugin interface
│       ├── ui/
│       │   ├── windows.lua      # Window management
│       │   ├── layout.lua       # Responsive layout
│       │   └── blocks.lua       # Visual operation blocks
│       ├── communication/
│       │   ├── grpc.lua         # gRPC client for Rust controller
│       │   └── protocol.lua     # Message protocol handling
│       ├── context/
│       │   ├── provider.lua     # Context gathering from Neovim
│       │   ├── files.lua        # File system context
│       │   └── diagnostics.lua  # Diagnostic information collection
│       └── config/
│           └── settings.lua     # Configuration management
├── plugin/                      # Neovim plugin registration
│   └── agent.vim                # Plugin registration and commands
├── container/                   # Rust-based MCP orchestration layer container
│   ├── Cargo.toml               # Rust dependencies for container binary
│   ├── src/
│   │   ├── main.rs              # Container entry point and gRPC server
│   │   ├── llm/
│   │   │   ├── mod.rs
│   │   │   ├── manager.rs       # LLM provider management (Ollama/OpenAI/Anthropic)
│   │   │   ├── ollama.rs        # Ollama integration
│   │   │   ├── openai.rs        # OpenAI API integration
│   │   │   └── anthropic.rs     # Anthropic API integration
│   │   ├── mcp/
│   │   │   ├── mod.rs
│   │   │   ├── engine.rs        # MCP orchestration engine
│   │   │   ├── client.rs        # MCP client for external services
│   │   │   └── protocol.rs      # MCP protocol handling
│   │   ├── spec/
│   │   │   ├── mod.rs
│   │   │   ├── engine.rs        # Spec-driven development engine
│   │   │   ├── requirements.rs  # Requirements management
│   │   │   ├── design.rs        # Design document management
│   │   │   └── tasks.rs         # Task management
│   │   ├── communication/
│   │   │   ├── mod.rs
│   │   │   ├── grpc.rs          # gRPC server implementation
│   │   │   └── protocol.rs      # Protocol definitions
│   │   ├── config/
│   │   │   ├── mod.rs
│   │   │   └── settings.rs      # Configuration management
│   │   └── utils/
│   │       ├── mod.rs
│   │       ├── error.rs         # Error types and handling
│   │       └── logging.rs       # Logging and diagnostics
│   └── config/
│       ├── llm_providers.json   # LLM provider configuration
│       └── mcp_services.json    # MCP service configuration
└── tests/
    ├── integration/             # End-to-end tests across all layers
    ├── unit/                    # Component-specific unit tests
    ├── property/                # Property-based tests
    └── container/               # Container-specific tests
```

### Component Communication Patterns

**Lua Frontend ↔ Rust Controller**
- Uses gRPC over local Unix socket or TCP for cross-platform compatibility
- Async request/response pattern with message correlation
- Health monitoring and automatic reconnection
- Request types: Chat, SpecOperation, CommandExecution, FileOperation, HealthCheck

**Rust Controller ↔ MCP Orchestration Layer**
- Uses gRPC over container networking
- Bidirectional streaming for real-time operations
- Container lifecycle management (start, stop, health checks)
- Context data serialization and transmission

**MCP Orchestration Layer ↔ LLM Providers**
- Ollama: HTTP API calls to local Ollama instance
- OpenAI: HTTPS API calls with secure API key management
- Anthropic: HTTPS API calls with secure API key management
- Unified interface regardless of provider

**MCP Orchestration Layer ↔ External MCP Services**
- Outbound connections only (container is not an MCP server)
- HTTP/WebSocket connections to external MCP-enabled services
- Authentication and authorization handling
- Graceful degradation when services are unavailable

### Neovim Integration Points

**Plugin Registration**
- Uses Neovim's Lua plugin architecture
- Registers commands and keybindings through Lua interface
- Provides health check function for `:checkhealth agent`
- Integrates with plugin managers through standard patterns

**Context Gathering**
- Leverages Neovim's Lua API for buffer and file system access
- Collects diagnostic information from LSP and other plugins
- Gathers project structure and workspace metadata
- Sanitizes and structures data before transmission

**UI Integration**
- Uses Neovim's floating window API for modern UI presentation
- Implements responsive layout calculations
- Provides visual feedback through buffer highlighting and virtual text
- Integrates with Neovim's event system for real-time updates

### Performance Considerations

**Memory Management**
- Implements conversation history limits with configurable retention
- Uses efficient data structures for large context data
- Provides memory usage monitoring across all components
- Container resource limits and cleanup mechanisms

**Async Operations**
- Non-blocking gRPC communication throughout the stack
- Async container operations to prevent UI freezing
- Background context gathering and processing
- Streaming responses for long-running operations

**Caching Strategy**
- Caches container health status and connection state
- Maintains parsed context data for quick access
- Implements intelligent conversation history indexing
- Container image caching for faster startup