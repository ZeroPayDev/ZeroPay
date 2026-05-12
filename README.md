# ZeroPay

<div align="center">

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Docker](https://img.shields.io/badge/docker-%230db7ed.svg?style=flat&logo=docker&logoColor=white)](https://hub.docker.com/r/zeropaydev/zeropay)
[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=flat&logo=rust&logoColor=white)](https://www.rust-lang.org)

An open-source, self-hosted x402 payment facilitator for AI agents and autonomous systems.

[Features](#features) • [Quick Start](#quick-start) • [Documentation](#documentation) • [Platform](#managed-platform) • [Contributing](#contributing)

</div>

---

## Overview

ZeroPay is a lightweight, self-hosted x402 payment facilitator built with Rust. It enables AI agents and autonomous systems to programmatically discover, authorize, and settle stablecoin payments using EIP-3009 gasless transfers — with no manual wallet management or blockchain infrastructure required.

### Key Features

- **x402 Protocol**: Full Agent-to-Agent (A2A) payment protocol for autonomous AI integrations
- **Gasless Payments**: EIP-3009 `transferWithAuthorization` — payer signs off-chain, payee covers gas
- **Instant Settlement**: No waiting for blockchain confirmation; payment settles in one transaction
- **Secure**: EIP-712 typed signatures with time-bound authorization windows
- **Discoverable**: Agents can browse available services via `/x402/discovery`
- **Multi-Chain**: Ethereum, Polygon, Base, and other EVM-compatible networks
- **Stablecoin Focused**: USDC, USDT, and other EIP-3009 compatible tokens
- **EIP-8004 Support**: Optional agent reputation and identity registry integration
- **Self-Hosted**: Full control over your payment infrastructure
- **Docker Ready**: One-command deployment

## Quick Start

### Using Docker Compose (Recommended)

1. **Configure your settings** in `docker-compose.yml`:
   ```yaml
   environment:
     - MNEMONICS=your twelve or twenty four word mnemonic phrase
     - WALLET=0xYourWalletAddress
     - APIKEY=your-secure-api-key
   ```

2. **Configure blockchain** in `config.toml`:
   ```toml
   [[chains]]
   chain_type = "evm"
   chain_name = "base-sepolia"
   latency = 1
   estimation = 12
   commission = 5
   commission_min = 50
   commission_max = 200
   admin = "0xYourAdminPrivateKey"
   rpc = "https://base-sepolia.g.alchemy.com/v2/YOUR-API-KEY"
   tokens = ["USDC:0xYourUSDCAddress:2"]
   ```
   The `:2` suffix marks a token as x402-compatible (EIP-3009).

3. **Start all services:**
   ```bash
   docker-compose up -d
   ```

4. **Verify x402 support:**
   ```bash
   curl "http://localhost:9000/x402/support?apikey=your-api-key"
   ```

See [DEPLOYMENT.md](./DEPLOYMENT.md) for detailed setup instructions.

## Documentation

- **[Deployment Guide](./DEPLOYMENT.md)** - Complete setup for Docker and source builds
- **[API Reference](./API.md)** - x402 REST API endpoints and usage examples
- **[x402 Protocol Guide](./x402.md)** - Protocol overview and integration details
- **[AI Integration Guide](./docs/AI_INTEGRATION_GUIDE.md)** - Complete guide for AI agents integrating with ZeroPay

## How x402 Works

```
┌─────────────────────────────────────────────────────┐
│                   AI Agent                          │
│                                                     │
│  1. GET /x402/requirements  →  discover payee addr  │
│  2. Sign EIP-712 authorization off-chain            │
│  3. POST /x402/payments     →  settle on-chain      │
│  4. Receive tx hash + confirmation                  │
└─────────────────────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────┐
│                 ZeroPay API                         │
│                                                     │
│  /x402/requirements   — payment requirements        │
│  /x402/payments       — verify + settle             │
│  /x402/support        — supported schemes           │
│  /x402/discovery      — browse available services   │
└───────────────────────┬─────────────────────────────┘
                        │ transferWithAuthorization
                        ▼
┌─────────────────────────────────────────────────────┐
│              Blockchain (EVM)                       │
│       EIP-3009 gasless token transfer               │
└─────────────────────────────────────────────────────┘
```

**Payment flow:**
1. Agent calls `GET /x402/requirements` with `customer` + `amount` → gets a payee address and accepted payment schemes
2. Agent creates an EIP-712 signature authorizing the transfer (off-chain, gasless for payer)
3. Agent calls `POST /x402/payments` with the signed authorization
4. ZeroPay verifies the signature and executes `transferWithAuthorization` on-chain
5. Returns transaction hash to the agent

## Architecture

```
zeropay/
├── api/              # REST API server (Axum)
├── scanner/          # Chain scanner + x402 asset initialization
├── x402/             # x402 protocol types, facilitator, client SDK
├── config.toml       # Chain and token configuration
├── Dockerfile
└── docker-compose.yml
```

## Development

### Prerequisites

- Rust 1.75+
- PostgreSQL 12+
- Redis 6+

### Build from Source

```bash
git clone https://github.com/zpaynow/zeropay.git
cd zeropay
cargo build --release
./target/release/api
```

## Managed Platform

For a hassle-free experience, use our managed platform at [zpaynow.com](https://zpaynow.com):

- No infrastructure management required
- Automatic updates and security patches
- Multiple chain support out of the box
- Enterprise-grade reliability

**Setup:** Register at [zpaynow.com](https://zpaynow.com) and use `https://api.zpaynow.com` as your API endpoint.

## Contributing

We welcome contributions!

### Reporting Vulnerabilities

Email hi@zpaynow.com instead of using the issue tracker.

### Best Practices

- Never commit `.env` files or private keys
- Use strong, randomly generated API keys
- Keep dependencies updated
- Use secure RPC endpoints

## License

This project is licensed under the GNU General Public License v3.0 - see the [LICENSE](LICENSE) file for details.

> **Usage Notice:** This software is intended for **merchant self-hosting only** — to run your own payment gateway for your own store or service. It may **not** be used to build or operate a payment business, payment platform, or SaaS product that serves other merchants. If you need a managed solution, use [zpaynow.com](https://zpaynow.com).
