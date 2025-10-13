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

### Key Features

- **Self-Hosted**: Full control over your payment infrastructure
- **Multi-Chain Support**: Compatible with Ethereum, Polygon, BSC, and other EVM chains
- **Stablecoin Focused**: Built for USDT, USDC, and other stablecoins
- **Real-Time Notifications**: Webhook integration for payment events
- **Automatic Settlement**: Funds automatically transferred to your wallet (minus commission)
- **Secure**: HMAC-based webhook authentication
- **Easy Integration**: RESTful API with comprehensive documentation
- **Docker Ready**: One-command deployment with Docker

## Quick Start

### Using Docker (Recommended)

```bash
# Pull the latest image
docker pull zeropaydev/zeropay:latest

# Create configuration
cp .env-template .env
# Edit .env with your settings

# Run the container
docker run -d \
  --name zeropay \
  -p 9000:9000 \
  --env-file .env \
  -v $(pwd)/config.toml:/app/config.toml \
  zeropaydev/zeropay:latest
```

### Using Docker Compose

```bash
# Start all services (PostgreSQL, Redis, ZeroPay)
docker-compose up -d
```

See [DEPLOYMENT.md](./DEPLOYMENT.md) for detailed setup instructions.

## Documentation

- **[Deployment Guide](./DEPLOYMENT.md)** - Complete setup instructions for Docker and source builds
- **[API Reference](./API.md)** - REST API endpoints and webhook events
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
┌─────────────┐
│   Client    │
│ Application │
└──────┬──────┘
       │ REST API
       ▼
┌─────────────┐
│   ZeroPay   │◄──────┐
│   API       │       │
└──────┬──────┘       │
       │              │
       ├──────────────┤
       │              │
       ▼              ▼
┌──────────┐   ┌──────────┐
│PostgreSQL│   │  Redis   │
└──────────┘   └──────────┘
       │
       │ Scanner
       ▼
┌─────────────────┐
│   Blockchain    │
│   (Ethereum,    │
│   Polygon, etc) │
└─────────────────┘
```

## Features

### Payment Processing
- Create unique payment addresses for each transaction
- Support for multiple stablecoins (USDT, USDC, DAI, etc.)
- Automatic payment detection and confirmation
- Configurable confirmation blocks for security

### Blockchain Support
- **EVM-Compatible Chains**: Ethereum, Polygon, BSC, Arbitrum, Optimism, Avalanche, etc.
- **Extensible**: Easy to add new chains via configuration
- **Multi-Token**: Support any ERC-20 token

### Webhook Events
- `session.paid` - Customer completed payment
- `session.settled` - Funds transferred to merchant
- `unknow.paid` - Unlinked payment received
- `unknow.settled` - Unlinked payment settled

### Security
- HMAC-SHA256 webhook signatures
- API key authentication
- HD wallet derivation for payment addresses
- Configurable confirmation requirements

## Configuration

### Environment Variables

Create a `.env` file from the template:

```bash
cp .env-template .env
```

Key configuration options:

```bash
PORT=9000                                           # API server port
DATABASE_URL=postgres://user:pass@localhost/zeropay # PostgreSQL connection
REDIS=redis://127.0.0.1:6379                        # Redis connection
MNEMONICS="your twelve or twenty-four word phrase"  # BIP39 seed phrase
WALLET=0xa0..00                                     # Settlement wallet address
APIKEY=your-secure-api-key                          # API authentication key
WEBHOOK=https://your-app.com/webhook                # Webhook endpoint URL
SCANNER_CONFIG=config.toml                          # Chain config file path
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

### Create Payment Session

```bash
curl -X POST "http://localhost:9000/sessions?apikey=your-api-key" \
  -H "Content-Type: application/json" \
  -d '{
    "customer": "user123",
    "amount": 1000
  }'
```

### Check Payment Status

```bash
curl "http://localhost:9000/sessions/12345?apikey=your-api-key"
```

See [API.md](./API.md) for complete API documentation.

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

If you discover a security vulnerability, please email security@zeropay.dev instead of using the issue tracker.

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

- **Documentation**: [DEPLOYMENT.md](./DEPLOYMENT.md) | [API.md](./API.md)
- **Issues**: [GitHub Issues](https://github.com/ZeroPayDev/zeropay/issues)
- **Discussions**: [GitHub Discussions](https://github.com/ZeroPayDev/zeropay/discussions)
- **Platform Support**: support@zeropay.dev

## Acknowledgments

Built with:
- [Rust](https://www.rust-lang.org/) - Systems programming language
- [Axum](https://github.com/tokio-rs/axum) - Web framework
- [Alloy](https://github.com/alloy-rs/alloy) - Ethereum library
- [SQLx](https://github.com/launchbadge/sqlx) - SQL toolkit
- [Redis](https://redis.io/) - In-memory data store

---

<div align="center">

Made with ❤️ by the ZeroPay community

[Website](https://zeropay.dev) • [GitHub](https://github.com/ZeroPayDev/zeropay) • [Twitter](https://twitter.com/zeropaydev)

</div>
