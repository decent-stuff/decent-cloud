# User Guide

This guide covers everything you need to know about using the Decent Cloud platform as a user.

## Getting Started

### Prerequisites

- Completed [installation](installation.md)
- Generated identity and registered account
- Initial DC tokens for operations

### Registration

```bash
dc user register --identity my-user
```

## Finding Resources

### Listing All Offerings

```bash
dc offering list
```

### Searching Specific Offerings

Use the query command with specific criteria:

```bash
dc offering query 'memory >= 512MB AND storage.size > 1gb'
```

Common search criteria:

- Memory: `memory >= <size>`
- Storage: `storage.size > <size>`
- CPU: `cpu.cores >= <number>`
- Location: `location == "<region>"`

### Understanding Offerings

Key aspects to consider:

1. Resource specifications
2. Provider reputation
3. Pricing
4. Location
5. Terms of service

## Contracting Resources

### Preparing for Contract

1. Review provider's profile and history:

```bash
dc np list --balances
```

2. Ensure sufficient token balance
3. Prepare SSH public key
4. Have contact information ready

### Creating a Contract Request

Basic contract request:

```bash
dc contract sign-request \
  --offering-id <offering-id> \
  --identity my-user \
  --requester-ssh-pubkey "ssh-ed25519 AAAAC3..." \
  --requester-contact "https://github.com/username" \
  --memo "Project deployment" \
  --provider-pubkey-pem <provider-key> \
  --interactive
```

> The `--interactive` flag will prompt for any missing required information.

### Monitoring Contract Status

Check open contract requests:

```bash
dc contract list-open
```

## Managing Resources

### Best Practices

1. **Resource Planning**

   - Accurately estimate needs
   - Consider scaling requirements
   - Plan for redundancy
   - Monitor usage patterns

2. **Cost Management**

   - Track token usage
   - Monitor resource utilization
   - Plan for long-term needs
   - Consider bulk contracts

3. **Security**
   - Use strong SSH keys
   - Implement access controls
   - Regular security audits
   - Backup critical data

## Token Management

### Checking Balance

```bash
dc ledger-remote fetch
dc user balance --identity my-user
```

### Token Best Practices

- Maintain sufficient balance for operations
- Monitor transaction history
- Keep private keys secure
- Regular ledger synchronization

## Troubleshooting

### Common Issues

1. **Connection Problems**

   - Verify network connectivity
   - Check SSH key configuration
   - Confirm provider status
   - Review firewall settings

2. **Resource Access**

   - Verify contract status
   - Check authentication
   - Review access permissions
   - Contact provider support

3. **Performance Issues**
   - Monitor resource usage
   - Check network latency
   - Review application logs
   - Document performance metrics

### Getting Support

1. **Provider Support**

   - Use provided contact methods
   - Document issues clearly
   - Follow up appropriately
   - Maintain professional communication

2. **Platform Support**
   - 📝 [Open an Issue](https://github.com/decent-stuff/decent-cloud/issues)
   - 💬 [Join Discussions](https://github.com/orgs/decent-stuff/discussions)
   - 📚 [Read Documentation](https://decent-cloud.org/)

## Best Practices

### Resource Usage

1. **Optimization**

   - Right-size resources
   - Implement auto-scaling
   - Monitor utilization
   - Regular performance reviews

2. **Maintenance**
   - Regular updates
   - Scheduled maintenance
   - Backup procedures
   - Disaster recovery plans

### Communication

1. **With Providers**

   - Clear requirements
   - Prompt responses
   - Professional conduct
   - Document interactions

2. **Issue Reporting**
   - Detailed descriptions
   - Reproducible steps
   - System information
   - Error logs

## Community Participation

### Getting Involved

- Share experiences
- Help other users
- Provide feedback
- Suggest improvements

### Contributing

- Report bugs
- Submit feature requests
- Share use cases
- Document solutions

## Future Features

Stay tuned for:

- WebUI interface
- Enhanced monitoring
- Automated resource management
- Advanced analytics

Remember: Your feedback helps improve the platform for everyone!
