# ZeroPay API Reference

ZeroPay implements the x402 Agent-to-Agent (A2A) payment protocol, enabling AI agents and autonomous systems to programmatically settle stablecoin payments using EIP-3009 gasless transfers.

## Table of Contents

- [Authentication](#authentication)
- [Base URL](#base-url)
- [Endpoints](#endpoints)
  - [GET /x402/requirements](#get-x402requirements)
  - [POST /x402/payments](#post-x402payments)
  - [GET /x402/support](#get-x402support)
  - [GET /x402/discovery](#get-x402discovery)
- [Response Codes](#response-codes)

## Authentication

All requests require an API key as a query parameter:

```
?apikey=your-api-key-here
```

## Base URL

### Self-Hosted
```
http://your-domain:9000
```

### Managed Platform
```
https://api.zpaynow.com
```

---

## Endpoints

### GET /x402/requirements

Discover payment requirements for a given customer and amount. An AI agent calls this first to learn the payee address and accepted payment schemes before constructing a payment authorization.

**Query Parameters:**
- `apikey` (required): Your API key

**Request Body:**
```json
{
  "customer": "string",
  "amount": integer
}
```

| Field | Type | Description |
|-------|------|-------------|
| `customer` | string | Unique identifier for the payer (e.g., agent ID, user ID) |
| `amount` | integer | Amount in cents (e.g., `1000` = $10.00) |

**Response:** `200 OK`
```json
{
  "accepts": [
    {
      "scheme": "exact",
      "network": "base-sepolia",
      "maxAmountRequired": "10.00",
      "resource": "https://...",
      "description": "",
      "mimeType": "",
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

**Example:**
```bash
curl -X GET "http://localhost:9000/x402/requirements?apikey=your-api-key" \
  -H "Content-Type: application/json" \
  -d '{"customer": "agent_alice", "amount": 1000}'
```

---

### POST /x402/payments

Submit a signed payment authorization. ZeroPay verifies the EIP-712 signature and executes `transferWithAuthorization` on-chain, returning the transaction hash.

**Query Parameters:**
- `apikey` (required): Your API key

**Request Body:**
```json
{
  "paymentPayload": {
    "x402Version": 1,
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
  "paymentRequirements": { ... }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `paymentPayload.x402Version` | integer | Protocol version (currently `1`) |
| `paymentPayload.scheme` | string | Payment scheme (`"exact"`) |
| `paymentPayload.network` | string | Target network (e.g., `"base-sepolia"`) |
| `paymentPayload.payload.signature` | string | EIP-712 signature from payer's wallet |
| `paymentPayload.payload.authorization.from` | string | Payer's EVM address |
| `paymentPayload.payload.authorization.to` | string | Payee's EVM address (from requirements) |
| `paymentPayload.payload.authorization.value` | string | Token amount in smallest unit (e.g., USDC uses 6 decimals) |
| `paymentPayload.payload.authorization.validAfter` | string | Unix timestamp — signature valid after this time |
| `paymentPayload.payload.authorization.validBefore` | string | Unix timestamp — signature expires at this time |
| `paymentPayload.payload.authorization.nonce` | string | Random 32-byte hex nonce |
| `paymentRequirements` | object | The full requirements object returned by `/x402/requirements` |

**Response — success:** `200 OK`
```json
{
  "success": true,
  "transaction": "0xabc123...",
  "network": "base-sepolia"
}
```

**Response — failure:**
```json
{
  "success": false,
  "error": "invalid signature",
  "network": "base-sepolia"
}
```

**Example:**
```bash
curl -X POST "http://localhost:9000/x402/payments?apikey=your-api-key" \
  -H "Content-Type: application/json" \
  -d '{
    "paymentPayload": {
      "x402Version": 1,
      "scheme": "exact",
      "network": "base-sepolia",
      "payload": {
        "signature": "0x...",
        "authorization": {
          "from": "0xPAYER",
          "to": "0xPAYEE",
          "value": "1000000",
          "validAfter": "0",
          "validBefore": "1735689600",
          "nonce": "0x..."
        }
      }
    },
    "paymentRequirements": { ... }
  }'
```

---

### GET /x402/support

List all payment schemes and networks currently supported by this ZeroPay instance.

**Query Parameters:**
- `apikey` (required): Your API key

**Response:** `200 OK`
```json
[
  {
    "scheme": "exact",
    "network": "base-sepolia",
    "x402Version": 1
  }
]
```

**Example:**
```bash
curl "http://localhost:9000/x402/support?apikey=your-api-key"
```

---

### GET /x402/discovery

Browse payment-enabled resources available through this facilitator. Agents can use this to find services they can pay for autonomously.

**Query Parameters:**
- `apikey` (required): Your API key
- `type` (optional): Filter by resource type
- `limit` (optional): Page size (default 20)
- `offset` (optional): Page offset (default 0)

**Response:** `200 OK`
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

**Example:**
```bash
curl "http://localhost:9000/x402/discovery?apikey=your-api-key&limit=10"
```

---

## Response Codes

| Code | Description |
|------|-------------|
| `200` | Success |
| `401` | Unauthorized — invalid or missing API key |
| `400` | Bad Request — malformed request body |
| `500` | Internal Server Error |

**Error response format:**
```json
{
  "status": "failure",
  "error": "error description"
}
```

## Support

- GitHub: [https://github.com/zpaynow/zeropay/issues](https://github.com/zpaynow/zeropay/issues)
- Platform Support: hi@zpaynow.com
