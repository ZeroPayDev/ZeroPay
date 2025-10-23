# ZeroPay

<div align="center">

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Docker](https://img.shields.io/badge/docker-%230db7ed.svg?style=flat&logo=docker&logoColor=white)](https://hub.docker.com/r/zeropaydev/zeropay)
[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=flat&logo=rust&logoColor=white)](https://www.rust-lang.org)

An open-source, self-hosted payment gateway for stablecoins and cryptocurrency payments.

[Features](#features) • [Quick Start](#quick-start) • [Documentation](#documentation) • [Platform](#managed-platform) • [Contributing](#contributing)

</div>

---

## Overview

ZeroPay is a lightweight, self-hosted payment gateway that enables merchants to accept stablecoin and cryptocurrency payments with minimal setup. Built with Rust for performance and reliability, it supports multiple EVM-compatible blockchains and provides real-time webhook notifications for payment events.

**New in v1.x:** ZeroPay now supports the **x402 Agent-to-Agent (A2A) payment protocol**, enabling AI agents and autonomous systems to programmatically discover, authorize, and settle payments using EIP-3009 gasless transfers.

### Key Features

- **Self-Hosted**: Full control over your payment infrastructure
- **Multi-Chain Support**: Compatible with Ethereum, Polygon, BSC, and other EVM chains
- **Stablecoin Focused**: Built for USDT, USDC, and other stablecoins
- **Real-Time Notifications**: Webhook integration for payment events
- **Automatic Settlement**: Funds automatically transferred to your wallet (minus commission)
- **x402 Protocol Support**: Agent-to-Agent (A2A) payment protocol for AI-powered integrations
- **Secure**: HMAC-based webhook authentication and EIP-712 signatures
- **Easy Integration**: RESTful API with comprehensive documentation
- **Docker Ready**: One-command deployment with Docker

## Quick Start

### Using Docker Compose (Recommended)

The easiest way to run ZeroPay with all dependencies:

1. **Configure your settings:**

   Edit `docker-compose.yml` environment variables:
   ```yaml
   environment:
     - MNEMONICS=your twelve or twenty four word mnemonic phrase
     - WALLET=0xYourWalletAddress
     - APIKEY=your-secure-api-key
     - WEBHOOK=https://your-webhook-url.com
   ```

2. **Configure blockchain:**

   Edit `config.toml` with your chain settings (RPC URL, tokens, etc.)

3. **Start all services:**
   ```bash
   docker-compose up -d
   ```

4. **Check logs:**
   ```bash
   docker-compose logs -f zeropay
   ```

### Using Docker Standalone

```bash
# Pull the latest image
docker pull zeropaydev/zeropay:latest

# Run with environment variables
docker run -d \
  --name zeropay \
  -p 9000:9000 \
  -e DATABASE_URL=postgres://user:pass@host:5432/zeropay \
  -e REDIS_URL=redis://host:6379 \
  -e MNEMONICS="your mnemonic phrase" \
  -e WALLET=0xYourWallet \
  -e APIKEY=your-api-key \
  -v $(pwd)/config.toml:/app/config.toml \
  zeropaydev/zeropay:latest
```

See [DEPLOYMENT.md](./DEPLOYMENT.md) for detailed setup instructions.

## Documentation

- **[Deployment Guide](./DEPLOYMENT.md)** - Complete setup instructions for Docker and source builds
- **[API Reference](./API.md)** - REST API endpoints and webhook events
- **[AI Integration Guide](./docs/AI_INTEGRATION_GUIDE.md)** - x402 A2A protocol implementation guide for AI agents
- **[Configuration Guide](#configuration)** - Environment and chain configuration

## Managed Platform

For a hassle-free experience, use our managed platform at [zeropay.dev](https://zeropay.dev):

**Benefits:**
- No infrastructure management required
- Automatic updates and security patches
- Public payment UI for customers
- Multiple chain support out of the box
- Enterprise-grade reliability

**Setup:**
1. Register your merchant account at [zeropay.dev](https://zeropay.dev)
2. Use `https://api.zeropay.dev` as your API endpoint
3. Start accepting payments immediately

**Note:** The platform charges a small commission for gas fees and hosting.

## Architecture

```
┌─────────────┐         ┌─────────────┐
│   Client    │         │  AI Agent   │
│ Application │         │  (x402)     │
└──────┬──────┘         └──────┬──────┘
       │ REST API              │ x402 Protocol
       │                       │ (EIP-3009)
       ▼                       ▼
┌──────────────────────────────────┐
│         ZeroPay API              │
│  /sessions     /x402/*           │◄──────┐
└──────┬───────────────────────────┘       │
       │                                   │
       ├───────────────────────────────────┤
       │                                   │
       ▼                                   ▼
┌──────────┐                        ┌──────────┐
│PostgreSQL│                        │  Redis   │
└──────────┘                        └──────────┘
       │
       │ Scanner + x402 Facilitator
       ▼
┌─────────────────────────────────────┐
│         Blockchain                  │
│  (Ethereum, Polygon, Base, etc)     │
│  EIP-3009 transferWithAuthorization │
└─────────────────────────────────────┘
```

## Features

### Payment Processing
- Create unique payment addresses for each transaction
- Support for multiple stablecoins (USDT, USDC, DAI, etc.)
- Automatic payment detection and confirmation
- Configurable confirmation blocks for security
- Support for both traditional and x402 A2A payment flows

### x402 Agent-to-Agent (A2A) Protocol
ZeroPay implements the x402 protocol, enabling AI agents and autonomous systems to interact with payment APIs programmatically:

- **Payment Requirements Discovery**: AI agents can query payment requirements and supported methods
- **EIP-3009 Authorization**: Gasless payment authorization using EIP-712 signatures
- **Automatic Verification**: Signature validation, balance checks, and transaction simulation
- **Instant Settlement**: Execute payments via `transferWithAuthorization` for immediate settlement
- **Resource Discovery**: Browse and discover available payment-enabled services

**x402 API Endpoints:**
- `GET /x402/requirements` - Get payment requirements for a resource
- `POST /x402/payments` - Submit payment authorization and settle
- `GET /x402/support` - List supported payment schemes and networks
- `GET /x402/discovery` - Discover available payment-enabled resources

See the [AI Integration Guide](./docs/AI_INTEGRATION_GUIDE.md) for complete x402 implementation details.

### Blockchain Support
- **EVM-Compatible Chains**: Ethereum, Polygon, BSC, Arbitrum, Optimism, Avalanche, etc.
- **Extensible**: Easy to add new chains via configuration
- **Multi-Token**: Support any ERC-20 token with EIP-3009 support

### Webhook Events
- `session.paid` - Customer completed payment
- `session.settled` - Funds transferred to merchant
- `unknow.paid` - Unlinked payment received
- `unknow.settled` - Unlinked payment settled

### Security
- HMAC-SHA256 webhook signatures
- EIP-712 signature verification for x402 payments
- API key authentication
- HD wallet derivation for payment addresses
- Configurable confirmation requirements

## Configuration

### Environment Variables

**For Docker Compose:** Edit the `environment` section in `docker-compose.yml`

**For local development:** Create a `.env` file or set as environment variables

Key configuration options:

```bash
PORT=9000                                            # API server port
DATABASE_URL=postgres://user:pass@host:5432/zeropay # PostgreSQL connection
REDIS_URL=redis://host:6379                          # Redis connection
MNEMONICS=your twelve or twenty-four word phrase     # BIP39 seed phrase
WALLET=0xa0..00                                      # Settlement wallet address
APIKEY=your-secure-api-key                           # API authentication key
WEBHOOK=https://your-app.com/webhook                 # Webhook endpoint URL
SCANNER_CONFIG=config.toml                           # Chain config file path
```

### Chain Configuration

Configure supported blockchains in `config.toml`:

```toml
[[chains]]
chain_type = "evm"
chain_name = "ethereum"
latency = 6                    # Confirmation blocks
estimation = 72                # Seconds to confirm
commission = 5                 # 5% commission
commission_min = 50            # $0.50 minimum
commission_max = 200           # $2.00 maximum
admin = "0xYourPrivateKey"     # Gas payment account
rpc = "https://eth-mainnet.g.alchemy.com/v2/YOUR-KEY"
tokens = [
  "USDT:0xdAC17F958D2ee523a2206206994597C13D831ec7",
  "USDC:0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
]
```

See [DEPLOYMENT.md](./DEPLOYMENT.md) for complete configuration details.

## API Usage

### Traditional Payment Flow

Create a payment session and receive a unique deposit address:

```bash
curl -X POST "http://localhost:9000/sessions?apikey=your-api-key" \
  -H "Content-Type: application/json" \
  -d '{
    "customer": "user123",
    "amount": 1000
  }'
```

Check payment status:

```bash
curl "http://localhost:9000/sessions/12345?apikey=your-api-key"
```

### x402 A2A Payment Flow

Query payment requirements for AI agents:

```bash
curl -X POST "http://localhost:9000/x402/requirements?apikey=your-api-key" \
  -H "Content-Type: application/json" \
  -d '{
    "customer": "agent_alice",
    "amount": 1000
  }'
```

Submit payment authorization and settle:

```bash
curl -X POST "http://localhost:9000/x402/payments?apikey=your-api-key" \
  -H "Content-Type: application/json" \
  -d '{
    "payment_payload": {
      "x402_version": 1,
      "scheme": "exact",
      "network": "base-sepolia",
      "payload": {
        "signature": "0x...",
        "authorization": {
          "from": "0x...",
          "to": "0x...",
          "value": "1000000",
          "validAfter": "0",
          "validBefore": "1735689600",
          "nonce": "0x..."
        }
      }
    },
    "payment_requirements": {...}
  }'
```

See [API.md](./API.md) for complete traditional API documentation and [AI_INTEGRATION_GUIDE.md](./docs/AI_INTEGRATION_GUIDE.md) for x402 protocol details.

## x402 Protocol Integration

The x402 protocol enables autonomous AI agents to interact with ZeroPay for programmatic payments. Key capabilities:

### How It Works

1. **Discovery**: AI agents query `/x402/requirements` to get payment requirements
2. **Authorization**: Agent creates an EIP-712 signature authorizing the payment
3. **Settlement**: ZeroPay verifies the signature and settles via `transferWithAuthorization`
4. **Response**: Agent receives transaction hash and settlement confirmation

### Key Benefits

- **Gasless Payments**: Payee covers gas fees, not the payer
- **Instant Settlement**: No waiting for blockchain confirmations
- **Secure**: EIP-712 signatures with time-bound authorization
- **Discoverable**: Agents can browse available services via `/x402/discovery`

### Supported Payment Schemes

- **exact**: EIP-3009 exact transfer authorization
- **Networks**: Base, Ethereum, Polygon, and other EVM chains
- **Tokens**: USDC and other EIP-3009 compatible tokens

### Client SDKs

For AI agent developers, use the ZeroPay x402 client SDK:

```rust
use x402::client::{ClientFacilitator, PaymentMethod};

// Initialize client with wallet
let facilitator = ClientFacilitator::new();
facilitator.add_payment_method(
    "base-sepolia",
    PaymentMethod::Evm(signer, rpc_url, tokens)
);

// Create payment payload
let payload = facilitator.create_payment(&requirements).await?;

// Submit payment
let response = facilitator.pay(&url, payload).await?;
```

See the complete [AI Integration Guide](./docs/AI_INTEGRATION_GUIDE.md) for implementation details.

## Development

### Prerequisites

- Rust 1.75 or higher
- PostgreSQL 12+
- Redis 6+

### Build from Source

```bash
# Clone the repository
git clone https://github.com/ZeroPayDev/zeropay.git
cd zeropay

# Build
cargo build --release

# Run
./target/release/api
```

### Project Structure

```
zeropay/
├── api/              # REST API server
├── scanner/          # Blockchain scanner
├── config.toml       # Chain configuration
├── Dockerfile        # Container build file
└── .env-template     # Environment template
```

## Contributing

We welcome contributions!

### Reporting Vulnerabilities

If you discover a security vulnerability, please email hi@zeropay.dev instead of using the issue tracker.

### Best Practices

- Never commit `.env` files or private keys
- Use strong, randomly generated API keys
- Verify webhook HMAC signatures
- Keep dependencies updated
- Use secure RPC endpoints
- Enable firewall rules

## License

This project is licensed under the GNU General Public License v3.0 - see the [LICENSE](LICENSE) file for details.

## Support

- **Documentation**: [DEPLOYMENT.md](./DEPLOYMENT.md) | [API.md](./API.md) | [AI Integration Guide](./docs/AI_INTEGRATION_GUIDE.md)
- **x402 Protocol**: [API Documentation](./docs/API_DOCUMENTATION.md) | [Specification](https://github.com/zeropaydev/x402)
- **Issues**: [GitHub Issues](https://github.com/ZeroPayDev/zeropay/issues)
- **Discussions**: [GitHub Discussions](https://github.com/ZeroPayDev/zeropay/discussions)
- **Platform Support**: hi@zeropay.dev

<div align="center">

Made with ❤️ by the ZeroPay community

[Website](https://zeropay.dev) • [GitHub](https://github.com/ZeroPayDev/zeropay) • [Twitter](https://twitter.com/zeropaydev)

</div>
