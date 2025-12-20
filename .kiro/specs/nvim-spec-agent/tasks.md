# Implementation Plan

- [x] 1. Comprehensive project review and architecture refactoring
  - Review existing codebase structure and identify components that need refactoring for three-layer architecture
  - Analyze current Lua-Rust communication and plan migration to gRPC-based system
  - Assess existing window management and UI components for compatibility with new architecture
  - Document current state and create refactoring roadmap for MCP orchestration layer integration
  - Identify reusable components and those requiring complete rewrite
  - _Requirements: 1.1, 1.2, 6.1, 8.1_

- [x] 1.1 Audit existing Rust backend implementation
  - Review current Rust modules (agent/, ui/, spec/, config/, utils/) for three-layer compatibility
  - Identify which components can be adapted vs need complete rewrite
  - Document current communication patterns and data structures
  - Plan migration strategy for existing functionality to new architecture
  - _Requirements: 8.1, 10.2_

- [x] 1.2 Audit existing Lua frontend implementation
  - Review current Lua modules and their integration with existing Rust backend
  - Assess window management and UI components for gRPC communication compatibility
  - Document current keybinding and command registration patterns
  - Plan context gathering implementation for MCP orchestration layer
  - _Requirements: 2.1, 2.2, 11.1, 11.2_

- [x] 1.3 Design container architecture for MCP orchestration layer
  - Create Docker container specification for MCP orchestration layer
  - Design LLM provider abstraction layer (Ollama, OpenAI, Anthropic)
  - Plan MCP client integration for external services
  - Design gRPC server interface for Rust controller communication
  - _Requirements: 6.2, 7.1, 12.1, 12.2_

- [ ]* 1.4 Write property test for architecture refactoring validation
  - **Property 1: Plugin installation with prebuilt binaries**
  - **Validates: Requirements 1.1, 1.2, 1.4, 1.5**

- [x] 2. Refactor Rust controller for container management
  - Refactor existing Rust backend to focus on container lifecycle management
  - Implement Docker container management (pull, start, stop, health monitoring)
  - Replace direct agent logic with gRPC client for container communication
  - Migrate existing configuration management to support container and LLM provider configs
  - Update error handling for container-specific scenarios
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5_

- [x] 2.1 Implement container lifecycle management
  - Create ContainerManager for Docker operations (pull, start, stop, cleanup)
  - Implement container health monitoring and automatic recovery
  - Add container configuration management for different LLM providers
  - Handle container startup failures with diagnostic information
  - _Requirements: 6.1, 6.2, 6.4, 6.5_

- [x] 2.2 Implement gRPC server for Lua frontend communication
  - Create gRPC server to replace existing Lua-Rust communication
  - Implement request routing to container via gRPC client
  - Add message correlation and async response handling
  - Implement connection health monitoring and retry logic
  - _Requirements: 8.1, 8.2, 8.3, 8.5_

- [x] 2.3 Implement gRPC client for container communication
  - Create gRPC client for communication with MCP orchestration layer
  - Implement bidirectional streaming for real-time operations
  - Add context data serialization and transmission
  - Handle container communication failures and recovery
  - _Requirements: 8.1, 8.2, 8.4, 8.5_

- [ ]* 2.4 Write property test for container lifecycle management
  - **Property 13: Docker container lifecycle management**
  - **Validates: Requirements 6.1, 6.2, 6.3, 6.4, 6.5**

- [x] 3. Refactor Lua frontend for gRPC communication
  - Update existing Lua modules to use gRPC instead of direct Rust communication
  - Implement context gathering from Neovim (buffers, files, diagnostics, project structure)
  - Refactor window management to work with new communication patterns
  - Update keybinding and command registration for new architecture
  - Migrate existing UI components to work with container-based responses
  - _Requirements: 2.1, 2.2, 8.1, 11.1, 11.2, 11.3, 11.4, 11.5_

- [x] 3.1 Implement gRPC client in Lua
  - Create Lua gRPC client for communication with Rust controller
  - Implement async request/response handling with callbacks
  - Add message queue for concurrent operations
  - Implement connection management and error recovery
  - _Requirements: 8.1, 8.2, 8.3_

- [x] 3.2 Implement context gathering system
  - Create context provider for current buffer contents and cursor position
  - Implement file system context gathering (paths, open files, project structure)
  - Add edit history and change tracking collection
  - Implement diagnostic information collection from LSP and plugins
  - Add context sanitization and structuring before transmission
  - _Requirements: 11.1, 11.2, 11.3, 11.4, 11.5_

- [x] 3.3 Refactor window management for new architecture
  - Update existing window management to work with gRPC responses
  - Implement real-time UI updates from container responses
  - Add progress indicators for container operations
  - Handle UI state synchronization with container state
  - _Requirements: 2.1, 2.2, 2.3, 2.5_

- [ ]* 3.4 Write property test for gRPC communication chain
  - **Property 15: gRPC communication chain integrity**
  - **Validates: Requirements 8.1, 8.2, 8.3, 8.4, 8.5**

- [x] 4. Implement Rust-based MCP orchestration layer container
  - Create a second Rust binary crate for the MCP orchestration layer
  - Set up Cargo workspace with container binary as separate crate
  - Implement Docker build to compile and containerize the Rust binary
  - Implement LLM provider abstraction (Ollama, OpenAI, Anthropic)
  - Create gRPC server for Rust controller communication
  - Implement MCP client for external service connections
  - Add secure API key management for cloud LLM providers
  - _Requirements: 6.2, 7.1, 7.2, 12.1, 12.2, 12.3, 12.4_

- [x] 4.1 Set up Rust container binary crate
  - Create container/Cargo.toml with dependencies (tonic, tokio, serde, etc.)
  - Create container/src/main.rs as entry point for gRPC server
  - Set up module structure (llm/, mcp/, spec/, communication/, config/, utils/)
  - Configure Cargo workspace to include container binary
  - Update Dockerfile to build Rust binary with multi-stage build
  - _Requirements: 6.2, 12.1_

- [x] 4.2 Implement LLM provider manager in Rust
  - Create unified trait-based interface for LLM providers
  - Implement Ollama provider with HTTP client integration
  - Add OpenAI provider with secure API key management
  - Implement Anthropic provider with authentication
  - Add provider switching and fallback mechanisms
  - Implement health monitoring for all providers
  - _Requirements: 12.1, 12.2, 12.3, 12.4, 12.5_

- [x] 4.3 Implement MCP orchestration engine in Rust
  - Create MCP orchestration engine with session management
  - Implement tool discovery and availability tracking
  - Add tool call extraction and execution routing
  - Implement context management and state persistence
  - Support streaming and non-streaming LLM responses
  - _Requirements: 7.1, 7.2, 9.1, 9.2_

- [x] 4.4 Implement container gRPC server in Rust
  - Create gRPC server using tonic framework
  - Implement request handlers for all request types (chat, spec, commands, files)
  - Handle conversation management and context processing
  - Implement streaming responses for long operations
  - Add health check and status endpoints
  - _Requirements: 8.1, 8.2, 10.5_

- [x] 4.5 Implement MCP client for external services in Rust
  - Create HTTP-based MCP client for external service connections
  - Implement multiple authentication types (API Key, Bearer, Basic, OAuth2)
  - Add tool discovery from external MCP services
  - Implement retry logic with exponential backoff
  - Add graceful degradation when services unavailable
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5_

- [ ]* 4.6 Write property test for MCP orchestration layer
  - **Property 14: MCP service integration**
  - **Validates: Requirements 7.1, 7.2, 7.3, 7.4, 7.5**

- [-] 5. Migrate existing spec-driven development features
  - Port existing spec workflow logic to MCP orchestration layer
  - Update requirements, design, and task management for new architecture
  - Implement property-based testing integration through MCP tools
  - Migrate existing command execution and approval workflows
  - Update file operation monitoring for container-based operations
  - _Requirements: 3.1, 3.2, 3.3, 4.1, 4.2, 5.1, 9.1, 9.2_

- [x] 5.1 Port spec workflow engine to container
  - Migrate existing spec workflow logic to MCP orchestration layer
  - Update EARS requirements validation for container environment
  - Port design document generation and correctness properties
  - Migrate task management and completion tracking
  - _Requirements: 3.1, 3.2, 3.3, 9.1, 9.2_

- [x] 5.2 Implement command execution through container
  - Port command approval workflow to work through MCP orchestration layer
  - Update command presentation and execution via gRPC
  - Implement output capture and delivery through container
  - Add error handling for container-based command execution
  - _Requirements: 4.2, 4.4, 4.5, 5.1, 5.2, 5.3, 5.4, 5.5_

- [-] 5.3 Update file operation monitoring
  - Port file operation visualization to work with container operations
  - Update progress indicators for container-based file operations
  - Implement real-time operation feedback through gRPC
  - Add visual blocks for container operation status
  - _Requirements: 4.1, 4.3_

- [ ]* 5.4 Write property test for migrated spec workflows
  - **Property 6: Spec workflow progression**
  - **Property 7: Spec navigation state preservation**
  - **Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5**

- [ ] 6. Implement enhanced context provision system
  - Complete implementation of Neovim context gathering for MCP orchestration layer
  - Add intelligent context filtering and prioritization
  - Implement context caching and incremental updates
  - Add context sanitization for security and privacy
  - Optimize context transmission for performance
  - _Requirements: 9.3, 9.4, 9.5, 11.1, 11.2, 11.3, 11.4, 11.5_

- [ ] 6.1 Implement comprehensive file context gathering
  - Gather current buffer contents, cursor position, and file metadata
  - Collect file system paths, open files, and project structure
  - Add recent edits, undo/redo history, and change tracking
  - Implement incremental context updates for performance
  - _Requirements: 11.1, 11.2, 11.3_

- [ ] 6.2 Implement debugging context collection
  - Collect compiler/linter output from LSP and plugins
  - Gather runtime errors and stack traces
  - Add diagnostic information aggregation
  - Implement context sanitization and structuring
  - _Requirements: 11.4, 11.5_

- [ ]* 6.3 Write property test for context provision
  - **Property 16: Enhanced agent capabilities with context provision**
  - **Validates: Requirements 9.3, 9.4, 9.5, 11.1, 11.2, 11.3, 11.4, 11.5**

- [ ] 7. Implement prebuilt binary distribution system
  - Update build system for prebuilt binary generation
  - Create CI/CD pipeline for multi-platform binary builds
  - Implement binary detection and validation in Lua frontend
  - Update plugin manager integration for prebuilt binaries
  - Add binary update and version management
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5_

- [ ] 7.1 Create multi-platform build system
  - Set up CI/CD for Linux, macOS, and Windows binary builds
  - Implement binary packaging and distribution to plugin-dist branch
  - Add binary signing and verification for security
  - Create automated release process
  - _Requirements: 1.1, 1.2, 1.4_

- [ ] 7.2 Update plugin manager integration
  - Update lazy.nvim configuration for prebuilt binary installation
  - Add support for packer.nvim and vim-plug
  - Implement binary validation and health checks
  - Add installation troubleshooting and diagnostics
  - _Requirements: 1.3, 1.4, 1.5_

- [ ]* 7.3 Write property test for prebuilt binary system
  - **Property 1: Plugin installation with prebuilt binaries**
  - **Property 2: Plugin loading and registration**
  - **Validates: Requirements 1.1, 1.2, 1.3, 1.4, 1.5**

- [ ] 8. Implement performance optimization and error handling
  - Optimize gRPC communication for low latency
  - Implement efficient memory management across all layers
  - Add comprehensive error handling and recovery mechanisms
  - Implement performance monitoring and diagnostics
  - Add graceful degradation for component failures
  - _Requirements: 10.1, 10.2, 10.3, 10.4, 10.5_

- [ ] 8.1 Optimize communication performance
  - Implement efficient gRPC message serialization
  - Add connection pooling and reuse
  - Optimize context data transmission
  - Implement streaming for large operations
  - _Requirements: 10.1, 10.5_

- [ ] 8.2 Implement comprehensive error handling
  - Add error recovery for container failures
  - Implement graceful degradation for MCP service failures
  - Add conversation history preservation during crashes
  - Implement diagnostic information collection
  - _Requirements: 10.4, 6.5, 7.5_

- [ ]* 8.3 Write property test for performance and reliability
  - **Property 17: Concurrent operation safety and performance**
  - **Validates: Requirements 10.1, 10.2, 10.3, 10.4, 10.5**

- [ ] 9. Integration testing and validation
  - Create comprehensive integration tests for three-layer architecture
  - Test end-to-end workflows with different LLM providers
  - Validate MCP service integration with external tools
  - Test plugin installation across different environments
  - Perform load testing and performance validation
  - _Requirements: All requirements validation_

- [ ] 9.1 Create end-to-end integration tests
  - Test complete workflows from Neovim to container and back
  - Validate spec-driven development workflows
  - Test command execution and approval workflows
  - Validate context gathering and provision
  - _Requirements: 3.1, 4.2, 5.1, 11.1_

- [ ] 9.2 Test LLM provider integrations
  - Test Ollama integration with local models
  - Validate OpenAI API integration with secure key management
  - Test Anthropic API integration
  - Validate provider switching and fallback mechanisms
  - _Requirements: 12.1, 12.2, 12.3, 12.4, 12.5_

- [ ] 9.3 Validate MCP service integration
  - Test external MCP service connections
  - Validate authentication and permission management
  - Test graceful degradation when services unavailable
  - Validate retry logic and error handling
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5_

- [ ] 10. Final integration and documentation
  - Complete final integration of all components
  - Create comprehensive user documentation
  - Add troubleshooting guides and FAQ
  - Implement final UI polish and user experience improvements
  - Create installation and configuration guides
  - _Requirements: 1.1, 2.1, 2.2_

- [ ] 10.1 Complete final integration
  - Integrate all components into cohesive plugin experience
  - Add final performance optimizations
  - Implement final UI polish and user experience improvements
  - Create comprehensive integration validation
  - _Requirements: 2.1, 2.2, 10.1_

- [ ] 10.2 Create comprehensive documentation
  - Write installation instructions for different plugin managers
  - Create LLM provider configuration guides
  - Add MCP service integration documentation
  - Create troubleshooting guides and FAQ
  - _Requirements: 1.1, 12.1, 7.1_

- [ ] 11. Final Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.