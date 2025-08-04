# provider-offering: Network Provider Offering Management

## Overview

The provider-offering library provides efficient management and discovery of compute offerings from network providers within the Decent Cloud ecosystem. It enables providers to publish their available services—including VPS, dedicated servers, and cloud instances—with detailed specifications, pricing, and availability information.

### Key Capabilities

- **Provider Identity**: Identify providers using 32-byte public keys (cryptographic validation is enabled but implemented elsewhere in the project)
- **Offering Discovery**: Find specific offerings quickly by provider and offering identifier
- **Service Search**: Allow users to search and filter offerings based on requirements
- **Market Efficiency**: Enable real-time updates of provider catalogs

## Core Functionality

### Provider Management

Network providers are identified by unique 32-byte public keys. Each provider maintains a catalog of server offerings with unique identifiers within their namespace. The system supports:

- **Adding new provider catalogs**: Import and register new provider offerings
- **Updating existing provider offerings**: Refresh and modify existing catalogs
- **Removing providers and their offerings**: Clean up deprecated or inactive providers

### Offering Structure

Each offering represents a specific compute service configuration with comprehensive metadata:

- **Identity**: Unique identifier within provider namespace, assigned by the provider
- **Specifications**: CPU, memory, storage, and network capabilities
- **Pricing**: Cost structure across different billing intervals
- **Location**: Geographic datacenter placement information
- **Availability**: Current stock status and provisioning details
- **Platform**: Supported operating systems and virtualization technologies

### Search and Discovery

The system provides multiple access patterns for finding offerings:

- **Direct Lookup**: O(1) retrieval by (provider_pubkey, offering_key) pair
- **Provider Browsing**: List all offerings from a specific provider
- **Text Search**: Find offerings matching keyword queries across descriptive fields
- **Structured Filtering**: Apply multiple criteria (price range, location, specifications)
- **Compound Queries**: Combine text search with structured filters for precise results

### Data Format

Offerings are exchanged in CSV format compatible with existing industry standards (serverhunter.com format). The format includes 35 standardized fields covering all relevant service attributes:

```
Offer Name, Description, Unique Internal identifier, Product page URL, Currency,
Monthly price, Setup fee, Visibility, Product Type, Virtualization type,
Billing interval, Stock, Processor Brand, Processor Amount, Processor Cores,
Processor Speed, Processor Name, Memory Error Correction, Memory Type,
Memory Amount, Hard Disk Drive Amount, Total Hard Disk Drive Capacity,
Solid State Disk Amount, Total Solid State Disk Capacity, Unmetered,
Uplink speed, Traffic, Datacenter Country, Datacenter City,
Datacenter Coordinates, Features, Operating Systems, Control Panel,
GPU Name, Payment Methods
```

## Serialization

### Motivation

Decent Cloud operates across multiple environments with different constraints: IC canisters have response size limits and favor compact formats, web frontends need JSON compatibility, and storage systems benefit from minimal overhead. A single serialization approach cannot efficiently serve all contexts.

### Design Approach

The library provides three specialized formats:

- **PEM + CSV**: Separates cryptographic identity (PEM-encoded Ed25519 keys) from structured data (CSV). Optimized for IC canister interfaces where size efficiency and Candid compatibility matter most.

- **Compact JSON**: Wraps PEM + CSV in JSON structure for web API compatibility while preserving internal efficiency gains.

Response size management ensures canister endpoints never exceed IC limits through progressive offering inclusion until size thresholds are reached.

## Technical Implementation

### Type System

Strong typing ensures data integrity and prevents common programming errors:

- **ProviderPubkey**: 32-byte array with compile-time size validation
- **Offering**: Combines provider context with server specification
- **OfferingKey**: String identifier unique within provider namespace (maps to `unique_internal_identifier`)

### Registry Architecture

A centralized registry manages all offerings with multiple indexing strategies:

- **Primary Index**: HashMap for O(1) direct lookups by (provider, key)
- **Provider Index**: Groups offerings by provider for efficient provider-scoped operations
- **Text Index**: Inverted keyword index for text search capabilities

### Search Implementation

The search engine uses a multi-stage pipeline to handle complex queries efficiently:

1. **Query Parsing**: Decompose search request into direct lookup, text, and filter components
2. **Candidate Selection**: Use most selective index to identify candidate set
3. **Filter Application**: Apply remaining filters to candidate set
4. **Pagination**: Apply offset/limit for large result sets

### Memory Management

The system optimizes memory usage through several strategies:

- **Single Storage**: Each offering stored once in primary index
- **Reference-based Indexes**: Secondary indexes store keys rather than duplicating data
- **Incremental Updates**: Provider updates replace only affected offerings

## Data Model

### Core Entities

- **ProviderPubkey**: 32-byte cryptographic identifier
- **Offering**: Complete offering specification
- **OfferingRegistry**: Offering indexes for quick search

### Query Model

**SearchQuery**: A declarative query specification that:
- Combines multiple filter types in a single request
- Supports progressive query building
- Enables basic query optimization through filter reordering

**OfferingFilter**: Typed filter predicates for:
- Price ranges, geographic constraints, and technical requirements
- Composable complex multi-criteria searches

## Integration Points

The registry is designed to integrate seamlessly with other components of the Decent Cloud ecosystem:

### Ledger Integration (External)

The registry integrates with the Decent Cloud ledger system (implemented elsewhere):
- Storing signed provider offerings with cryptographic verification
- Tracking offering update fees and provider reputation
- Maintaining offering history for audit and rollback

### CLI Integration (External)

Command-line tools (implemented elsewhere) provide:
- Provider offering management (add, update, remove)
- Search and discovery capabilities
- Import/export utilities

### Web Interface (External)

Browser-based marketplace functionality (implemented elsewhere):
- Interactive offering browsing and comparison
- Real-time availability updates

## Quality Attributes

### Maintainability

- Clear separation between data model and search logic
- Basic test coverage for core operations

### Security

- Cryptographic validation of provider signatures (enabled but implemented elsewhere)
- Input sanitization for search queries

## Future Work

### Advanced Features

The following features are planned for future implementation:

### Enhanced Registry Architecture

- **Filter Index**: Secondary indexes to accelerate common filtering operations
- **Lazy Indexing**: Text indexes built on-demand for active search fields

### Query Optimization

- **Advanced Query Optimization**: More sophisticated predicate analysis and query planning
- **Result Ranking**: Sort results by relevance for better user experience

### Real-time Updates

- **Market Efficiency**: Real-time updates of provider catalogs through event-driven architecture
- **Change Notification**: Notify consumers of offering changes in real-time

### Extensibility

- **Extensible Filter System**: Plugin-based filter system for easy addition of new filter types
- **Comprehensive Test Coverage**: Expanded test suite covering edge cases and error conditions

### Performance

- **Performance Targets**: Specific performance benchmarks and optimizations for large-scale deployments
- **Memory Optimization**: Advanced memory management techniques for reduced footprint
