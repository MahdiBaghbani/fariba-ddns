# Provider Setup Guide

This guide explains how to set up and configure different DNS providers with Fariba DDNS Client.

## Supported Providers

- [Cloudflare](#cloudflare)
- [ArvanCloud](#arvancloud)

## Cloudflare

### Prerequisites
- A Cloudflare account
- A domain managed by Cloudflare
- API token with DNS edit permissions

### Setup Steps

1. **Create API Token**
   - Go to Cloudflare Dashboard > Profile > API Tokens
   - Click "Create Token"
   - Use "Edit zone DNS" template or create custom with:
     - Zone - DNS - Edit
     - Zone - Zone - Read
   - Restrict to specific zones if desired
   - Copy the generated token

2. **Get Zone ID**
   - Go to your domain's overview page
   - Zone ID is shown in the right sidebar
   - Copy the Zone ID

3. **Configuration**
   ```toml
   [providers.cloudflare]
   api_token = "your-api-token"
   zone_id = "your-zone-id"
   domains = [
     "example.com",      # apex domain
     "*.example.com",    # wildcard
     "sub.example.com"   # specific subdomain
   ]
   ```

### Permissions
Minimum required permissions for the API token:
- `Zone.DNS.Edit`
- `Zone.Zone.Read`

### Rate Limits
- 1200 requests per 5 minutes
- Client automatically respects these limits

## ArvanCloud

### Prerequisites
- An ArvanCloud account
- A domain managed by ArvanCloud
- API key with DNS management permissions

### Setup Steps

1. **Create API Key**
   - Log in to ArvanCloud Panel
   - Go to Profile > API Keys
   - Create a new API key
   - Copy the generated key

2. **Configuration**
   ```toml
   [providers.arvancloud]
   api_key = "your-api-key"
   domains = [
     "example.ir",
     "sub.example.ir"
   ]
   ```

### Permissions
The API key needs:
- DNS record management permissions

### Rate Limits
- 120 requests per minute
- Client automatically handles rate limiting

## Common Configuration Tips

### Domain Patterns
- Use `example.com` for apex domain
- Use `*.example.com` for wildcard subdomains
- Use `sub.example.com` for specific subdomains

### Security Best Practices
1. Use environment variables for credentials
2. Create dedicated API tokens/keys for DDNS
3. Restrict permissions to minimum required
4. Regularly rotate credentials

### Troubleshooting

1. **Authentication Errors**
   - Verify API token/key is correct
   - Check token permissions
   - Ensure token hasn't expired

2. **DNS Update Errors**
   - Verify domain ownership
   - Check domain name format
   - Ensure DNS record exists

3. **Rate Limiting**
   - Increase update interval
   - Check for other applications using same API token
   - Monitor API usage in provider dashboard 