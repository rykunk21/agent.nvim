"""
LLM Provider Manager

Manages different LLM providers (Ollama, OpenAI, Anthropic) with a unified interface.
Handles provider selection, fallback mechanisms, and health monitoring.
"""

import asyncio
import json
import os
from abc import ABC, abstractmethod
from dataclasses import dataclass
from enum import Enum
from typing import Any, AsyncIterator, Dict, List, Optional, Union

import structlog
from pydantic import BaseModel, Field

logger = structlog.get_logger(__name__)


class ProviderType(Enum):
    """Supported LLM provider types."""
    OLLAMA = "ollama"
    OPENAI = "openai"
    ANTHROPIC = "anthropic"


class ProviderStatus(Enum):
    """Provider health status."""
    HEALTHY = "healthy"
    UNHEALTHY = "unhealthy"
    UNKNOWN = "unknown"


@dataclass
class Message:
    """Standard message format for LLM communication."""
    role: str  # "user", "assistant", "system"
    content: str
    metadata: Optional[Dict[str, Any]] = None


@dataclass
class Response:
    """Standard response format from LLM providers."""
    content: str
    metadata: Optional[Dict[str, Any]] = None
    finish_reason: Optional[str] = None


@dataclass
class ProviderCapabilities:
    """Capabilities supported by a provider."""
    supports_streaming: bool = False
    supports_function_calling: bool = False
    supports_vision: bool = False
    max_tokens: Optional[int] = None
    context_window: Optional[int] = None


class ProviderConfig(BaseModel):
    """Base configuration for LLM providers."""
    enabled: bool = True
    timeout: int = 30
    max_retries: int = 3


class LLMProvider(ABC):
    """Abstract base class for LLM providers."""
    
    def __init__(self, config: Dict[str, Any]):
        self.config = config
        self.status = ProviderStatus.UNKNOWN
        self.last_error: Optional[str] = None
    
    @abstractmethod
    async def initialize(self) -> None:
        """Initialize the provider."""
        pass
    
    @abstractmethod
    async def generate_response(
        self, 
        messages: List[Message], 
        context: Optional[Dict[str, Any]] = None
    ) -> Response:
        """Generate a response from the LLM."""
        pass
    
    @abstractmethod
    async def stream_response(
        self, 
        messages: List[Message], 
        context: Optional[Dict[str, Any]] = None
    ) -> AsyncIterator[Response]:
        """Stream a response from the LLM."""
        pass
    
    @abstractmethod
    def get_capabilities(self) -> ProviderCapabilities:
        """Get provider capabilities."""
        pass
    
    @abstractmethod
    async def health_check(self) -> bool:
        """Check if the provider is healthy."""
        pass
    
    async def shutdown(self) -> None:
        """Shutdown the provider."""
        pass


class OllamaProvider(LLMProvider):
    """Ollama provider for local models."""
    
    def __init__(self, config: Dict[str, Any]):
        super().__init__(config)
        self.endpoint = config.get("endpoint", "http://localhost:11434")
        self.default_model = config.get("default_model", "llama2")
        self.client = None
    
    async def initialize(self) -> None:
        """Initialize Ollama client."""
        try:
            # Import ollama client if available
            import httpx
            self.client = httpx.AsyncClient(base_url=self.endpoint)
            
            # Test connection
            await self.health_check()
            logger.info("Ollama provider initialized", endpoint=self.endpoint)
            
        except ImportError:
            logger.error("Ollama client not available. Install with: pip install ollama-python")
            raise
        except Exception as e:
            logger.error("Failed to initialize Ollama provider", error=str(e))
            self.status = ProviderStatus.UNHEALTHY
            self.last_error = str(e)
            raise
    
    async def generate_response(
        self, 
        messages: List[Message], 
        context: Optional[Dict[str, Any]] = None
    ) -> Response:
        """Generate response using Ollama."""
        if not self.client:
            raise RuntimeError("Ollama provider not initialized")
        
        try:
            # Convert messages to Ollama format
            ollama_messages = [
                {"role": msg.role, "content": msg.content} 
                for msg in messages
            ]
            
            # Make request to Ollama
            response = await self.client.post("/api/chat", json={
                "model": self.default_model,
                "messages": ollama_messages,
                "stream": False
            })
            response.raise_for_status()
            
            result = response.json()
            self.status = ProviderStatus.HEALTHY
            
            return Response(
                content=result["message"]["content"],
                metadata={"model": self.default_model, "provider": "ollama"}
            )
            
        except Exception as e:
            logger.error("Ollama generation failed", error=str(e))
            self.status = ProviderStatus.UNHEALTHY
            self.last_error = str(e)
            raise
    
    async def stream_response(
        self, 
        messages: List[Message], 
        context: Optional[Dict[str, Any]] = None
    ) -> AsyncIterator[Response]:
        """Stream response using Ollama."""
        if not self.client:
            raise RuntimeError("Ollama provider not initialized")
        
        try:
            # Convert messages to Ollama format
            ollama_messages = [
                {"role": msg.role, "content": msg.content} 
                for msg in messages
            ]
            
            # Make streaming request to Ollama
            async with self.client.stream("POST", "/api/chat", json={
                "model": self.default_model,
                "messages": ollama_messages,
                "stream": True
            }) as response:
                response.raise_for_status()
                
                async for line in response.aiter_lines():
                    if line:
                        try:
                            data = json.loads(line)
                            if "message" in data and "content" in data["message"]:
                                yield Response(
                                    content=data["message"]["content"],
                                    metadata={"model": self.default_model, "provider": "ollama"},
                                    finish_reason=data.get("done_reason")
                                )
                        except json.JSONDecodeError:
                            continue
            
            self.status = ProviderStatus.HEALTHY
            
        except Exception as e:
            logger.error("Ollama streaming failed", error=str(e))
            self.status = ProviderStatus.UNHEALTHY
            self.last_error = str(e)
            raise
    
    def get_capabilities(self) -> ProviderCapabilities:
        """Get Ollama capabilities."""
        return ProviderCapabilities(
            supports_streaming=True,
            supports_function_calling=False,
            supports_vision=False,
            context_window=4096  # Default, varies by model
        )
    
    async def health_check(self) -> bool:
        """Check Ollama health."""
        if not self.client:
            return False
        
        try:
            response = await self.client.get("/api/tags")
            response.raise_for_status()
            self.status = ProviderStatus.HEALTHY
            return True
        except Exception as e:
            self.status = ProviderStatus.UNHEALTHY
            self.last_error = str(e)
            return False
    
    async def shutdown(self) -> None:
        """Shutdown Ollama client."""
        if self.client:
            await self.client.aclose()
            self.client = None


class OpenAIProvider(LLMProvider):
    """OpenAI provider for cloud models."""
    
    def __init__(self, config: Dict[str, Any]):
        super().__init__(config)
        self.api_key = os.getenv(config.get("api_key_env", "OPENAI_API_KEY"))
        self.default_model = config.get("default_model", "gpt-4")
        self.base_url = config.get("base_url")
        self.max_tokens = config.get("max_tokens", 4096)
        self.client = None
    
    async def initialize(self) -> None:
        """Initialize OpenAI client."""
        if not self.api_key:
            raise ValueError("OpenAI API key not found in environment")
        
        try:
            from openai import AsyncOpenAI
            self.client = AsyncOpenAI(
                api_key=self.api_key,
                base_url=self.base_url
            )
            
            # Test connection
            await self.health_check()
            logger.info("OpenAI provider initialized", model=self.default_model)
            
        except ImportError:
            logger.error("OpenAI client not available. Install with: pip install openai")
            raise
        except Exception as e:
            logger.error("Failed to initialize OpenAI provider", error=str(e))
            self.status = ProviderStatus.UNHEALTHY
            self.last_error = str(e)
            raise
    
    async def generate_response(
        self, 
        messages: List[Message], 
        context: Optional[Dict[str, Any]] = None
    ) -> Response:
        """Generate response using OpenAI."""
        if not self.client:
            raise RuntimeError("OpenAI provider not initialized")
        
        try:
            # Convert messages to OpenAI format
            openai_messages = [
                {"role": msg.role, "content": msg.content} 
                for msg in messages
            ]
            
            # Make request to OpenAI
            response = await self.client.chat.completions.create(
                model=self.default_model,
                messages=openai_messages,
                max_tokens=self.max_tokens
            )
            
            self.status = ProviderStatus.HEALTHY
            
            return Response(
                content=response.choices[0].message.content,
                metadata={
                    "model": self.default_model, 
                    "provider": "openai",
                    "usage": response.usage.dict() if response.usage else None
                },
                finish_reason=response.choices[0].finish_reason
            )
            
        except Exception as e:
            logger.error("OpenAI generation failed", error=str(e))
            self.status = ProviderStatus.UNHEALTHY
            self.last_error = str(e)
            raise
    
    async def stream_response(
        self, 
        messages: List[Message], 
        context: Optional[Dict[str, Any]] = None
    ) -> AsyncIterator[Response]:
        """Stream response using OpenAI."""
        if not self.client:
            raise RuntimeError("OpenAI provider not initialized")
        
        try:
            # Convert messages to OpenAI format
            openai_messages = [
                {"role": msg.role, "content": msg.content} 
                for msg in messages
            ]
            
            # Make streaming request to OpenAI
            stream = await self.client.chat.completions.create(
                model=self.default_model,
                messages=openai_messages,
                max_tokens=self.max_tokens,
                stream=True
            )
            
            async for chunk in stream:
                if chunk.choices and chunk.choices[0].delta.content:
                    yield Response(
                        content=chunk.choices[0].delta.content,
                        metadata={"model": self.default_model, "provider": "openai"},
                        finish_reason=chunk.choices[0].finish_reason
                    )
            
            self.status = ProviderStatus.HEALTHY
            
        except Exception as e:
            logger.error("OpenAI streaming failed", error=str(e))
            self.status = ProviderStatus.UNHEALTHY
            self.last_error = str(e)
            raise
    
    def get_capabilities(self) -> ProviderCapabilities:
        """Get OpenAI capabilities."""
        return ProviderCapabilities(
            supports_streaming=True,
            supports_function_calling=True,
            supports_vision=True,
            max_tokens=self.max_tokens,
            context_window=8192  # Varies by model
        )
    
    async def health_check(self) -> bool:
        """Check OpenAI health."""
        if not self.client:
            return False
        
        try:
            # Simple test request
            await self.client.models.list()
            self.status = ProviderStatus.HEALTHY
            return True
        except Exception as e:
            self.status = ProviderStatus.UNHEALTHY
            self.last_error = str(e)
            return False


class AnthropicProvider(LLMProvider):
    """Anthropic provider for Claude models."""
    
    def __init__(self, config: Dict[str, Any]):
        super().__init__(config)
        self.api_key = os.getenv(config.get("api_key_env", "ANTHROPIC_API_KEY"))
        self.default_model = config.get("default_model", "claude-3-sonnet-20240229")
        self.max_tokens = config.get("max_tokens", 4096)
        self.client = None
    
    async def initialize(self) -> None:
        """Initialize Anthropic client."""
        if not self.api_key:
            raise ValueError("Anthropic API key not found in environment")
        
        try:
            from anthropic import AsyncAnthropic
            self.client = AsyncAnthropic(api_key=self.api_key)
            
            # Test connection
            await self.health_check()
            logger.info("Anthropic provider initialized", model=self.default_model)
            
        except ImportError:
            logger.error("Anthropic client not available. Install with: pip install anthropic")
            raise
        except Exception as e:
            logger.error("Failed to initialize Anthropic provider", error=str(e))
            self.status = ProviderStatus.UNHEALTHY
            self.last_error = str(e)
            raise
    
    async def generate_response(
        self, 
        messages: List[Message], 
        context: Optional[Dict[str, Any]] = None
    ) -> Response:
        """Generate response using Anthropic."""
        if not self.client:
            raise RuntimeError("Anthropic provider not initialized")
        
        try:
            # Convert messages to Anthropic format
            anthropic_messages = [
                {"role": msg.role, "content": msg.content} 
                for msg in messages
            ]
            
            # Make request to Anthropic
            response = await self.client.messages.create(
                model=self.default_model,
                messages=anthropic_messages,
                max_tokens=self.max_tokens
            )
            
            self.status = ProviderStatus.HEALTHY
            
            return Response(
                content=response.content[0].text,
                metadata={
                    "model": self.default_model, 
                    "provider": "anthropic",
                    "usage": response.usage.dict() if hasattr(response, 'usage') else None
                },
                finish_reason=response.stop_reason
            )
            
        except Exception as e:
            logger.error("Anthropic generation failed", error=str(e))
            self.status = ProviderStatus.UNHEALTHY
            self.last_error = str(e)
            raise
    
    async def stream_response(
        self, 
        messages: List[Message], 
        context: Optional[Dict[str, Any]] = None
    ) -> AsyncIterator[Response]:
        """Stream response using Anthropic."""
        if not self.client:
            raise RuntimeError("Anthropic provider not initialized")
        
        try:
            # Convert messages to Anthropic format
            anthropic_messages = [
                {"role": msg.role, "content": msg.content} 
                for msg in messages
            ]
            
            # Make streaming request to Anthropic
            async with self.client.messages.stream(
                model=self.default_model,
                messages=anthropic_messages,
                max_tokens=self.max_tokens
            ) as stream:
                async for text in stream.text_stream:
                    yield Response(
                        content=text,
                        metadata={"model": self.default_model, "provider": "anthropic"}
                    )
            
            self.status = ProviderStatus.HEALTHY
            
        except Exception as e:
            logger.error("Anthropic streaming failed", error=str(e))
            self.status = ProviderStatus.UNHEALTHY
            self.last_error = str(e)
            raise
    
    def get_capabilities(self) -> ProviderCapabilities:
        """Get Anthropic capabilities."""
        return ProviderCapabilities(
            supports_streaming=True,
            supports_function_calling=False,
            supports_vision=True,
            max_tokens=self.max_tokens,
            context_window=200000  # Claude 3 context window
        )
    
    async def health_check(self) -> bool:
        """Check Anthropic health."""
        if not self.client:
            return False
        
        try:
            # Simple test request (this might need adjustment based on Anthropic API)
            test_messages = [{"role": "user", "content": "Hello"}]
            await self.client.messages.create(
                model=self.default_model,
                messages=test_messages,
                max_tokens=1
            )
            self.status = ProviderStatus.HEALTHY
            return True
        except Exception as e:
            self.status = ProviderStatus.UNHEALTHY
            self.last_error = str(e)
            return False


class LLMProviderManager:
    """Manages multiple LLM providers with fallback and health monitoring."""
    
    def __init__(self, config_path: str = "/app/config/llm_providers.json"):
        self.config_path = config_path
        self.providers: Dict[str, LLMProvider] = {}
        self.config: Dict[str, Any] = {}
        self.default_provider: Optional[str] = None
        self.fallback_providers: List[str] = []
    
    async def initialize(self) -> None:
        """Initialize all configured providers."""
        logger.info("Initializing LLM provider manager")
        
        # Load configuration
        await self._load_config()
        
        # Initialize providers
        for provider_name, provider_config in self.config.get("providers", {}).items():
            if not provider_config.get("enabled", False):
                logger.info("Skipping disabled provider", provider=provider_name)
                continue
            
            try:
                provider = self._create_provider(provider_name, provider_config)
                await provider.initialize()
                self.providers[provider_name] = provider
                logger.info("Provider initialized", provider=provider_name)
                
            except Exception as e:
                logger.error("Failed to initialize provider", 
                           provider=provider_name, error=str(e))
        
        # Set default and fallback providers
        self.default_provider = self.config.get("default_provider")
        self.fallback_providers = self.config.get("fallback_providers", [])
        
        if not self.providers:
            raise RuntimeError("No LLM providers available")
        
        logger.info("LLM provider manager initialized", 
                   providers=list(self.providers.keys()),
                   default=self.default_provider)
    
    async def _load_config(self) -> None:
        """Load provider configuration."""
        try:
            with open(self.config_path, 'r') as f:
                self.config = json.load(f)
        except FileNotFoundError:
            logger.warning("Provider config not found, using defaults", path=self.config_path)
            self.config = {
                "providers": {
                    "ollama": {
                        "enabled": True,
                        "type": "ollama",
                        "config": {"endpoint": "http://localhost:11434"}
                    }
                },
                "default_provider": "ollama"
            }
        except Exception as e:
            logger.error("Failed to load provider config", error=str(e))
            raise
    
    def _create_provider(self, name: str, config: Dict[str, Any]) -> LLMProvider:
        """Create a provider instance."""
        provider_type = config.get("type")
        provider_config = config.get("config", {})
        
        if provider_type == "ollama":
            return OllamaProvider(provider_config)
        elif provider_type == "openai":
            return OpenAIProvider(provider_config)
        elif provider_type == "anthropic":
            return AnthropicProvider(provider_config)
        else:
            raise ValueError(f"Unknown provider type: {provider_type}")
    
    async def generate_response(
        self, 
        messages: List[Message], 
        context: Optional[Dict[str, Any]] = None,
        provider_name: Optional[str] = None
    ) -> Response:
        """Generate response using specified or default provider."""
        provider = self._get_provider(provider_name)
        
        try:
            return await provider.generate_response(messages, context)
        except Exception as e:
            logger.error("Provider failed, trying fallback", 
                        provider=provider_name or self.default_provider, 
                        error=str(e))
            
            # Try fallback providers
            for fallback_name in self.fallback_providers:
                if fallback_name in self.providers and fallback_name != (provider_name or self.default_provider):
                    try:
                        fallback_provider = self.providers[fallback_name]
                        return await fallback_provider.generate_response(messages, context)
                    except Exception as fallback_error:
                        logger.error("Fallback provider failed", 
                                   provider=fallback_name, 
                                   error=str(fallback_error))
            
            # All providers failed
            raise RuntimeError("All LLM providers failed")
    
    async def stream_response(
        self, 
        messages: List[Message], 
        context: Optional[Dict[str, Any]] = None,
        provider_name: Optional[str] = None
    ) -> AsyncIterator[Response]:
        """Stream response using specified or default provider."""
        provider = self._get_provider(provider_name)
        
        try:
            async for response in provider.stream_response(messages, context):
                yield response
        except Exception as e:
            logger.error("Provider streaming failed", 
                        provider=provider_name or self.default_provider, 
                        error=str(e))
            raise
    
    def _get_provider(self, provider_name: Optional[str] = None) -> LLMProvider:
        """Get provider by name or default."""
        name = provider_name or self.default_provider
        
        if not name or name not in self.providers:
            raise ValueError(f"Provider not available: {name}")
        
        return self.providers[name]
    
    def get_provider_status(self) -> Dict[str, Dict[str, Any]]:
        """Get status of all providers."""
        status = {}
        for name, provider in self.providers.items():
            status[name] = {
                "status": provider.status.value,
                "last_error": provider.last_error,
                "capabilities": provider.get_capabilities().__dict__
            }
        return status
    
    async def health_check_all(self) -> Dict[str, bool]:
        """Health check all providers."""
        results = {}
        for name, provider in self.providers.items():
            try:
                results[name] = await provider.health_check()
            except Exception as e:
                logger.error("Health check failed", provider=name, error=str(e))
                results[name] = False
        return results
    
    async def shutdown(self) -> None:
        """Shutdown all providers."""
        logger.info("Shutting down LLM provider manager")
        
        for name, provider in self.providers.items():
            try:
                await provider.shutdown()
                logger.info("Provider shutdown", provider=name)
            except Exception as e:
                logger.error("Provider shutdown failed", provider=name, error=str(e))
        
        self.providers.clear()
        logger.info("LLM provider manager shutdown complete")