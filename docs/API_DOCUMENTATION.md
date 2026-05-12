# ZeroPay x402 API Documentation

ZeroPay exposes four REST endpoints implementing the x402 Agent-to-Agent (A2A) payment protocol. All endpoints require an API key and operate over standard HTTP/JSON.

## Table of Contents

1. [Getting Started](#getting-started)
2. [Authentication](#authentication)
3. [Endpoints](#endpoints)
4. [x402 Payment Flow](#x402-payment-flow)
5. [Error Handling](#error-handling)
6. [Best Practices](#best-practices)
7. [FAQ](#faq)

---

## Getting Started

### What is ZeroPay?

ZeroPay is an x402 payment facilitator. It allows AI agents and autonomous systems to pay for resources programmatically using EIP-3009 gasless token transfers. The payer signs an off-chain authorization; ZeroPay executes the on-chain settlement.

### Requirements

- An API key
- An EVM-compatible wallet with USDC/USDT (or other supported EIP-3009 tokens)
- An RPC-accessible EVM chain (Base, Ethereum, Polygon, etc.)

### Base URL

```
https://api.zpaynow.com
```

---

## Authentication

All requests require an API key as a query parameter:

```
?apikey=your_api_key_here
```

Keep your API key secret. Never expose it in client-side code or public repositories.

---

## Endpoints

### GET /x402/requirements

Get payment requirements for a customer and amount. Returns the payee address and accepted payment schemes needed to construct an authorization.

**Request:**
```http
GET /x402/requirements?apikey={API_KEY}
Content-Type: application/json

{
  "customer": "agent_alice",
  "amount": 1000
}
```

| Field | Type | Description |
|-------|------|-------------|
| `customer` | string | Unique identifier for the payer |
| `amount` | integer | Amount in cents (1000 = $10.00) |

**Response:**
```json
{
  "accepts": [
    {
      "scheme": "exact",
      "network": "base-sepolia",
      "maxAmountRequired": "10.00",
      "payToAddress": "0xAbCd...",
      "requiredDeadlineSeconds": 300,
      "x402Version": 1,
      "extra": {
        "name": "USD Coin",
        "version": "2"
      }
    }
  ]
}
```

| Field | Description |
|-------|-------------|
| `scheme` | Payment scheme (`"exact"` for EIP-3009) |
| `network` | Target blockchain network |
| `maxAmountRequired` | Maximum amount in USD |
| `payToAddress` | Merchant's EVM address — use as `to` in your authorization |
| `requiredDeadlineSeconds` | Max seconds for `validBefore - now` |
| `x402Version` | Protocol version |
| `extra.name` / `extra.version` | EIP-712 domain info for signing |

---

### POST /x402/payments

Submit a signed EIP-3009 payment authorization. ZeroPay verifies the signature and calls `transferWithAuthorization` on-chain.

**Request:**
```http
POST /x402/payments?apikey={API_KEY}
Content-Type: application/json

{
  "paymentPayload": {
    "x402Version": 1,
    "scheme": "exact",
    "network": "base-sepolia",
    "payload": {
      "signature": "0x...",
      "authorization": {
        "from": "0xPayerAddress",
        "to": "0xPayeeAddress",
        "value": "1000000",
        "validAfter": "0",
        "validBefore": "1735689600",
        "nonce": "0x..."
      }
    }
  },
  "paymentRequirements": { ... }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `x402Version` | integer | Must be `1` |
| `scheme` | string | Must be `"exact"` |
| `network` | string | Must match requirements `network` |
| `payload.signature` | string | EIP-712 signature from payer |
| `authorization.from` | string | Payer's address |
| `authorization.to` | string | `payToAddress` from requirements |
| `authorization.value` | string | Token amount (USDC: 6 decimals) |
| `authorization.validAfter` | string | Unix timestamp (use `"0"` for immediate) |
| `authorization.validBefore` | string | Unix timestamp (use `now + requiredDeadlineSeconds`) |
| `authorization.nonce` | string | Random 32-byte hex nonce |
| `paymentRequirements` | object | Full object returned by `/x402/requirements` |

**Success response:**
```json
{
  "success": true,
  "transaction": "0xabc123...",
  "network": "base-sepolia"
}
```

**Failure response:**
```json
{
  "success": false,
  "error": "invalid signature",
  "network": "base-sepolia"
}
```

---

### GET /x402/support

Returns a list of all payment schemes and networks this instance supports.

**Request:**
```http
GET /x402/support?apikey={API_KEY}
```

**Response:**
```json
[
  {
    "scheme": "exact",
    "network": "base-sepolia",
    "x402Version": 1
  }
]
```

---

### GET /x402/discovery

Browse payment-enabled resources available through this facilitator. Supports pagination.

**Request:**
```http
GET /x402/discovery?apikey={API_KEY}&type=api&limit=10&offset=0
```

| Param | Type | Description |
|-------|------|-------------|
| `type` | string | Optional resource type filter |
| `limit` | integer | Page size (default 20) |
| `offset` | integer | Page offset (default 0) |

**Response:**
```json
{
  "resources": [
    {
      "url": "https://...",
      "description": "...",
      "schemes": ["exact"],
      "networks": ["base-sepolia"]
    }
  ],
  "total": 1
}
```

---

## x402 Payment Flow

```
Agent                         ZeroPay                        Blockchain
  │                              │                               │
  │── GET /x402/requirements ───►│                               │
  │◄── payToAddress + scheme ────│                               │
  │                              │                               │
  │  [sign EIP-712 off-chain]    │                               │
  │                              │                               │
  │── POST /x402/payments ──────►│                               │
  │                              │── transferWithAuthorization ──►│
  │                              │◄── tx hash ───────────────────│
  │◄── { success, transaction } ─│                               │
```

**Key properties of this flow:**
- Payer signs off-chain — no gas cost at signing time
- ZeroPay calls `transferWithAuthorization` — payee covers gas
- One HTTP round-trip from requirements to settlement
- No session state, no polling, no webhooks

---

## Error Handling

### HTTP Errors

| Code | Meaning | Action |
|------|---------|--------|
| `200` | Success (check `success` field for payment result) | — |
| `400` | Malformed request body | Fix request format |
| `401` | Invalid or missing API key | Check `?apikey=` parameter |
| `500` | Server error | Retry with backoff |

### Error Response Format

```json
{
  "status": "failure",
  "error": "user auth error"
}
```

### Payment-Level Errors (HTTP 200, `success: false`)

| `error` value | Cause | Action |
|---------------|-------|--------|
| `"invalid signature"` | EIP-712 signing error or wrong domain | Recheck domain/types |
| `"nonce already used"` | Nonce was previously submitted | Generate a new nonce |
| `"expired"` | `validBefore` is in the past | Increase deadline |
| `"amount mismatch"` | `value` doesn't match `maxAmountRequired` | Recheck token decimals |

---

## Best Practices

### Signing

- Always derive `validBefore` from `requiredDeadlineSeconds` returned in requirements
- Use `crypto.randomBytes(32)` or equivalent for nonces — never reuse
- Verify token decimals before computing `value` (USDC: 6, USDT: 6)

### Reliability

- Implement retry for nonce collisions (generate new nonce, retry immediately)
- Do not retry `"invalid signature"` — fix the signing logic
- For 500 errors, retry with exponential backoff

### Security

- Store private keys in environment variables or a secrets manager
- Never log signatures or authorization objects
- Rotate API keys periodically

---

## FAQ

**Q: What tokens are supported?**

A: USDC, USDT, and any EIP-3009 compatible token configured in `config.toml`. Call `/x402/support` to see what's active on your instance.

---

**Q: How do I know which token address to use for signing?**

A: The `/x402/requirements` response `extra` field includes the token name and EIP-712 version. Map these to your known token addresses, or read `config.toml` directly if self-hosting.

---

**Q: What happens if I submit the same nonce twice?**

A: The second submission returns `{ "success": false, "error": "nonce already used" }`. Generate a fresh random nonce and resubmit.

---

**Q: Does the payer need gas?**

A: No. The payer signs off-chain. ZeroPay (as the facilitator/payee) calls `transferWithAuthorization` and pays the gas.

---

**Q: Do you support testnet?**

A: Yes. Configure a testnet chain in `config.toml` (e.g., `base-sepolia`). Use testnet USDC for development.

---

**Q: Can one agent pay for multiple customers?**

A: Yes. Use a different `customer` string per payee context. Each customer gets a deterministic deposit address derived from the merchant's mnemonics.

---

**Happy building with ZeroPay!**
