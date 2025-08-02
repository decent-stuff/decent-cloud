# np-offering: Network Provider Offering Management

## Motivation

The Decent Cloud ecosystem requires efficient management and discovery of compute offerings from network providers. Providers publish their available services (VPS, dedicated servers, cloud instances) with detailed specifications, pricing, and availability information. The platform needs to support:

- **Provider Identity**: Cryptographically identify providers using 32-byte public keys
- **Offering Discovery**: Find specific offerings quickly by provider and offering identifier
- **Service Search**: Allow users to search and filter offerings based on requirements
- **Market Efficiency**: Enable real-time updates of provider catalogs
- **Data Integrity**: Ensure offering authenticity through cryptographic signatures

## Functional Requirements

### Provider Management
Network providers are identified by unique 32-byte public keys. Each provider maintains a catalog of server offerings with unique identifiers within their namespace. The system must support:

- Adding new provider catalogs
- Updating existing provider offerings
- Removing providers and their offerings
- Validating provider identity through cryptographic signatures

### Offering Structure
Each offering represents a specific compute service configuration with comprehensive metadata including:

- **Identity**: Unique identifier within provider namespace, assigned by the provider
- **Specifications**: CPU, memory, storage, network capabilities
- **Pricing**: Cost structure across different billing intervals
- **Location**: Geographic datacenter placement
- **Availability**: Current stock status and provisioning details
- **Platform**: Supported operating systems and virtualization

### Search and Discovery
The system provides multiple access patterns:

- **Direct Lookup**: O(1) retrieval by (provider_pubkey, offering_key) pair
- **Provider Browsing**: List all offerings from a specific provider
- **Text Search**: Find offerings matching keyword queries across descriptive fields
- **Structured Filtering**: Apply multiple criteria (price range, location, specifications)
- **Compound Queries**: Combine text search with structured filters

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

## Technical Approach

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
- **Filter Index**: Secondary indexes to accelerate common filtering operations

### Search Implementation
Multi-stage search pipeline supports complex queries:

1. **Query Parsing**: Decompose search request into direct lookup, text, and filter components
2. **Candidate Selection**: Use most selective index to identify candidate set
3. **Filter Application**: Apply remaining filters to candidate set
4. **Result Ranking**: Sort results by relevance (optional)
5. **Pagination**: Apply offset/limit for large result sets

### Memory Management
Efficient memory usage through:

- **Single Storage**: Each offering stored once in primary index
- **Reference-based Indexes**: Secondary indexes store keys rather than duplicating data
- **Incremental Updates**: Provider updates replace only affected offerings
- **Lazy Indexing**: Text indexes built on-demand for active search fields

### Performance Targets
Designed for current scale with growth capacity:

- **Capacity**: 1000+ providers, 10,000+ total offerings
- **Lookup Latency**: <1ms for direct access, <10ms for complex searches
- **Memory Footprint**: <1MB for 1000 offerings including all indexes
- **Update Performance**: <100ms to refresh entire provider catalog

## Data Model

### Core Entities

- **ProviderPubkey**: 32-byte cryptographic identifier
- **Offering**: Complete offering specification
- **OfferingRegistry**: Offering indexes for quick search

### Query Model

**SearchQuery**: Declarative query specification
- Combines multiple filter types in single request
- Supports progressive query building
- Enables query optimization through predicate analysis

**OfferingFilter**: Typed filter predicates
- Price ranges, geographic constraints, technical requirements
- Composable for complex multi-criteria searches
- Extensible for future filter types

## Integration Points

### Ledger Integration
The registry integrates with the Decent Cloud ledger system for:
- Storing signed provider offerings with cryptographic verification
- Tracking offering update fees and provider reputation
- Maintaining offering history for audit and rollback

### CLI Integration
Command-line tools provide:
- Provider offering management (add, update, remove)
- Search and discovery capabilities
- Import/export utilities

### Web Interface
Browser-based marketplace functionality:
- Interactive offering browsing and comparison
- Real-time availability updates

## Quality Attributes

### Maintainability
- Clear separation between data model and search logic
- Extensible filter system for future requirements
- Comprehensive test coverage for all operations

### Security
- Cryptographic validation of provider signatures
- Input sanitization for search queries
