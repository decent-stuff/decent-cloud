# Provider Guide

This comprehensive guide covers everything you need to know about being a provider on the Decent Cloud platform.

## Getting Started as a Provider

### Prerequisites

- Completed [installation](installation.md)
- Generated identity and registered account
- Initial DC tokens for operations

### Registration

```bash
dc provider register --identity my-provider
```

## Managing Your Provider Profile

### Profile Setup

1. Create a profile file following [the template](https://github.com/decent-stuff/decent-cloud/blob/main/examples/provider-profile-template.yaml)
2. Validate your YAML (use tools like [yamllint.com](https://www.yamllint.com/))
3. Update your profile:

```bash
dc provider update-profile --identity my-provider --profile-file my-provider-profile.yaml
```

> Note: A small fee is required for profile updates to prevent DoS attacks.

### Profile Best Practices

- Keep information accurate and up-to-date
- Provide detailed contact information
- Include relevant certifications and credentials
- Regularly review and update your profile

## Managing Offerings

### Creating an Offering

1. Prepare your offering YAML file following [the template](https://github.com/decent-stuff/decent-cloud/blob/main/examples/offering-example.csv)
2. Validate the CSV structure
3. Publish your offering:

```bash
dc provider update-offering --identity my-provider --offering-file my-provider-offering.csv
```

### Offering Guidelines

- Be specific about resource specifications
- Clearly state limitations and conditions
- Set competitive pricing
- Include all relevant technical details

## Handling Contracts

### Reviewing Contract Requests

Check for open contract requests:

```bash
dc contract list-open
```

### Accepting/Rejecting Contracts

Process a contract request:

```bash
dc contract sign-reply --identity my-provider --contract-id <contract-id-base64> --sign-accept true --response-text "Welcome aboard!" --interactive
```

Rejection example:

```bash
dc contract sign-reply --identity my-provider --contract-id <contract-id-base64> --sign-accept false --response-text "Resources currently unavailable" --interactive
```

### Contract Management Best Practices

1. **Evaluation Criteria**

   - Check user reputation
   - Verify resource availability
   - Review contract terms
   - Assess technical requirements

2. **Response Time**

   - Maintain quick response times
   - Keep users informed of status
   - Document decision reasoning

3. **Resource Allocation**
   - Ensure resources are ready before accepting
   - Monitor resource usage
   - Maintain service quality

## Participating in Token Distribution

Providers earn DCT rewards through blockchain validation (also called "check-ins" or "mining").

### Recommended: Automated Docker Validation

**The recommended way to participate is using the automated Docker deployment:**

```bash
# See cf/README.md for complete setup instructions
python3 cf/deploy.py deploy prod
```

This automatically validates the blockchain every 10 minutes, earning you a share of block rewards.

**Benefits:**
- ✅ Runs continuously without manual intervention
- ✅ Automatic validation every 10 minutes
- ✅ Built-in monitoring and health checks
- ✅ Easy to set up and maintain

See [cf/README.md](../cf/README.md#blockchain-validator-optional) and [mining-and-validation.md](mining-and-validation.md) for detailed setup instructions.

### Manual Check-ins (CLI)

For testing or one-off validations:

```bash
dc provider check-in --identity my-id --memo "Active and serving customers!"
```

**Note:** Manual validation requires running the command every 10 minutes. The Docker deployment handles this automatically.

### Best Practices

- Use Docker deployment for continuous automated validation
- Maintain sufficient DCT balance for validation fees (0.5 DCT per validation)
- Monitor validator health: `docker logs decent-cloud-api-validate-prod`
- Keep your local ledger synchronized
- Track reward distribution regularly

## Monitoring and Maintenance

### Ledger Synchronization

```bash
dc ledger-remote fetch
```

### Balance Checking

```bash
dc provider list --balances
```

### System Health

- Regularly monitor resource usage
- Keep systems updated
- Maintain security measures

## Security Best Practices for Providers

1. **Platform Key Management**

   - Store Decent Cloud identity mnemonics securely (consider hardware security modules)
   - Regular backup of `~/.dcc/identity/` directory
   - Limit access to provider identity keys to authorized personnel

2. **Reputation System Security**

   - Maintain consistent check-ins to preserve reputation score
   - Promptly address contract disputes to maintain positive rating
   - Monitor provider balance: `dc provider list --balances`

3. **Resource Security**

   - Isolate tenant environments using containers or VMs
   - Implement rate limiting for resource access
   - Log all contract-related operations for audit trails

## Reputation Management

### Building Reputation

- Maintain high service quality
- Quick response to issues
- Professional communication
- Regular system maintenance

### Handling Issues

- Prompt problem resolution
- Clear communication
- Fair refund policies
- Document incident responses

## Support and Community

For provider-specific support and community discussions:
- 📝 [Open an Issue](https://github.com/decent-stuff/decent-cloud/issues)
- 💬 [Join Discussions](https://github.com/orgs/decent-stuff/discussions)

