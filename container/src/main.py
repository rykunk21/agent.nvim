#!/usr/bin/env python3
"""
MCP Orchestration Layer Entry Point

This is the main entry point for the Docker container that hosts the MCP orchestration layer.
It initializes all components and starts the gRPC server for communication with the Rust controller.
"""

import asyncio
import logging
import os
import signal
import sys
from typing import Optional

import structlog
from grpc import aio as grpc_aio

from .communication.grpc_server import AgentServicer, create_grpc_server
from .orchestration.llm_manager import LLMProviderManager
from .orchestration.mcp_engine import MCPOrchestrationEngine
from .orchestration.spec_engine import SpecEngine
from .mcp.client import MCPClient


# Configure structured logging
structlog.configure(
    processors=[
        structlog.stdlib.filter_by_level,
        structlog.stdlib.add_logger_name,
        structlog.stdlib.add_log_level,
        structlog.stdlib.PositionalArgumentsFormatter(),
        structlog.processors.TimeStamper(fmt="iso"),
        structlog.processors.StackInfoRenderer(),
        structlog.processors.format_exc_info,
        structlog.processors.UnicodeDecoder(),
        structlog.processors.JSONRenderer()
    ],
    context_class=dict,
    logger_factory=structlog.stdlib.LoggerFactory(),
    wrapper_class=structlog.stdlib.BoundLogger,
    cache_logger_on_first_use=True,
)

logger = structlog.get_logger(__name__)


class MCPOrchestrationContainer:
    """Main container class that orchestrates all components."""
    
    def __init__(self):
        self.grpc_server: Optional[grpc_aio.Server] = None
        self.llm_manager: Optional[LLMProviderManager] = None
        self.mcp_engine: Optional[MCPOrchestrationEngine] = None
        self.spec_engine: Optional[SpecEngine] = None
        self.mcp_client: Optional[MCPClient] = None
        self.shutdown_event = asyncio.Event()
        
    async def initialize(self) -> None:
        """Initialize all components."""
        logger.info("Initializing MCP orchestration container")
        
        try:
            # Initialize LLM provider manager
            self.llm_manager = LLMProviderManager()
            await self.llm_manager.initialize()
            logger.info("LLM provider manager initialized")
            
            # Initialize MCP client for external services
            self.mcp_client = MCPClient()
            await self.mcp_client.initialize()
            logger.info("MCP client initialized")
            
            # Initialize MCP orchestration engine
            self.mcp_engine = MCPOrchestrationEngine(
                llm_manager=self.llm_manager,
                mcp_client=self.mcp_client
            )
            await self.mcp_engine.initialize()
            logger.info("MCP orchestration engine initialized")
            
            # Initialize spec engine
            self.spec_engine = SpecEngine(
                llm_manager=self.llm_manager,
                mcp_engine=self.mcp_engine
            )
            await self.spec_engine.initialize()
            logger.info("Spec engine initialized")
            
            # Create and start gRPC server
            servicer = AgentServicer(
                llm_manager=self.llm_manager,
                mcp_engine=self.mcp_engine,
                spec_engine=self.spec_engine,
                mcp_client=self.mcp_client
            )
            
            self.grpc_server = await create_grpc_server(servicer)
            logger.info("gRPC server created and started")
            
        except Exception as e:
            logger.error("Failed to initialize container", error=str(e), exc_info=True)
            raise
    
    async def run(self) -> None:
        """Run the container until shutdown."""
        logger.info("Starting MCP orchestration container")
        
        # Set up signal handlers
        loop = asyncio.get_running_loop()
        for sig in (signal.SIGTERM, signal.SIGINT):
            loop.add_signal_handler(sig, self._signal_handler)
        
        try:
            # Wait for shutdown signal
            await self.shutdown_event.wait()
        except Exception as e:
            logger.error("Error during container execution", error=str(e), exc_info=True)
        finally:
            await self.shutdown()
    
    def _signal_handler(self) -> None:
        """Handle shutdown signals."""
        logger.info("Received shutdown signal")
        self.shutdown_event.set()
    
    async def shutdown(self) -> None:
        """Gracefully shutdown all components."""
        logger.info("Shutting down MCP orchestration container")
        
        try:
            # Stop gRPC server
            if self.grpc_server:
                logger.info("Stopping gRPC server")
                await self.grpc_server.stop(grace=5.0)
                self.grpc_server = None
            
            # Shutdown components in reverse order
            if self.spec_engine:
                await self.spec_engine.shutdown()
                self.spec_engine = None
                logger.info("Spec engine shutdown")
            
            if self.mcp_engine:
                await self.mcp_engine.shutdown()
                self.mcp_engine = None
                logger.info("MCP orchestration engine shutdown")
            
            if self.mcp_client:
                await self.mcp_client.shutdown()
                self.mcp_client = None
                logger.info("MCP client shutdown")
            
            if self.llm_manager:
                await self.llm_manager.shutdown()
                self.llm_manager = None
                logger.info("LLM provider manager shutdown")
                
        except Exception as e:
            logger.error("Error during shutdown", error=str(e), exc_info=True)
        
        logger.info("MCP orchestration container shutdown complete")


async def main() -> None:
    """Main entry point."""
    # Configure logging level from environment
    log_level = os.getenv("LOG_LEVEL", "INFO").upper()
    logging.basicConfig(level=getattr(logging, log_level, logging.INFO))
    
    logger.info("Starting MCP orchestration container", 
                version="0.1.0", 
                python_version=sys.version,
                grpc_port=os.getenv("GRPC_PORT", "50051"))
    
    container = MCPOrchestrationContainer()
    
    try:
        await container.initialize()
        await container.run()
    except KeyboardInterrupt:
        logger.info("Received keyboard interrupt")
    except Exception as e:
        logger.error("Container failed", error=str(e), exc_info=True)
        sys.exit(1)
    finally:
        await container.shutdown()
    
    logger.info("Container stopped")


if __name__ == "__main__":
    # Ensure we're running in an async context
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\nShutdown requested by user")
    except Exception as e:
        print(f"Fatal error: {e}")
        sys.exit(1)