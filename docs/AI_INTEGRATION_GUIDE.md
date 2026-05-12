# ZeroPay AI Agent Integration Guide

This guide explains how AI agents and autonomous systems integrate with ZeroPay using the x402 Agent-to-Agent (A2A) payment protocol.

## Overview

ZeroPay implements the x402 protocol on top of EIP-3009 gasless token transfers. An agent:

1. **Discovers** payment requirements (payee address, amount, network)
2. **Signs** an EIP-712 authorization off-chain (no gas required from payer)
3. **Submits** the signed authorization to ZeroPay
4. **Receives** a transaction hash confirming on-chain settlement

No session management, no polling, no webhooks — one round-trip from requirements to settled payment.

---

## API Base Configuration

```javascript
const ZEROPAY_CONFIG = {
  apiUrl: "https://api.zpaynow.com",  // or your self-hosted URL
  apiKey: "your-api-key-here",
};
```

---

## Step 1: Get Payment Requirements

Call `GET /x402/requirements` to discover the payee address and accepted payment schemes.

### Request
```http
GET /x402/requirements?apikey={API_KEY}
Content-Type: application/json

{
  "customer": "agent_alice",
  "amount": 1000
}
```

`amount` is in cents (1000 = $10.00).

### Response
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
      "extra": { "name": "USD Coin", "version": "2" }
    }
  ]
}
```

### Implementation Examples

#### JavaScript/TypeScript
```javascript
async function getPaymentRequirements(customer, amountInCents) {
  const res = await fetch(
    `${ZEROPAY_CONFIG.apiUrl}/x402/requirements?apikey=${ZEROPAY_CONFIG.apiKey}`,
    {
      method: 'GET',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ customer, amount: amountInCents }),
    }
  );
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

const requirements = await getPaymentRequirements("agent_alice", 1000);
const payTo = requirements.accepts[0].payToAddress;
```

#### Python
```python
import requests

def get_payment_requirements(customer: str, amount_cents: int) -> dict:
    res = requests.get(
        f"{ZEROPAY_CONFIG['apiUrl']}/x402/requirements",
        params={"apikey": ZEROPAY_CONFIG['apiKey']},
        json={"customer": customer, "amount": amount_cents},
    )
    res.raise_for_status()
    return res.json()

requirements = get_payment_requirements("agent_alice", 1000)
pay_to = requirements["accepts"][0]["payToAddress"]
```

#### Go
```go
type RequirementsReq struct {
    Customer string `json:"customer"`
    Amount   int    `json:"amount"`
}

func getPaymentRequirements(customer string, amount int) (map[string]any, error) {
    body, _ := json.Marshal(RequirementsReq{Customer: customer, Amount: amount})
    url := fmt.Sprintf("%s/x402/requirements?apikey=%s", config.ApiUrl, config.ApiKey)
    resp, err := http.Get(url) // attach body via http.NewRequest in production
    if err != nil {
        return nil, err
    }
    defer resp.Body.Close()
    var result map[string]any
    json.NewDecoder(resp.Body).Decode(&result)
    return result, nil
}
```

#### Rust (with the built-in x402 client SDK)
```rust
use x402::client::{ClientFacilitator, PaymentMethod};

let facilitator = ClientFacilitator::new();
facilitator.add_payment_method(
    "base-sepolia",
    PaymentMethod::Evm(signer, rpc_url, tokens),
);

// The client SDK handles requirements fetching and signing internally:
let payload = facilitator.create_payment(&requirements).await?;
```

---

## Step 2: Sign the Payment Authorization

The agent signs an EIP-712 `TransferWithAuthorization` message using its private key. No gas is spent at signing time.

**EIP-712 domain:**
```json
{
  "name": "USD Coin",
  "version": "2",
  "chainId": 84532,
  "verifyingContract": "0xTokenAddress"
}
```

**Typed data:**
```json
{
  "TransferWithAuthorization": [
    { "name": "from",        "type": "address" },
    { "name": "to",          "type": "address" },
    { "name": "value",       "type": "uint256" },
    { "name": "validAfter",  "type": "uint256" },
    { "name": "validBefore", "type": "uint256" },
    { "name": "nonce",       "type": "bytes32" }
  ]
}
```

**Values:**
- `from`: agent's wallet address
- `to`: `payToAddress` from requirements
- `value`: token amount in smallest units (USDC: 6 decimals, so $10.00 = `10000000`)
- `validAfter`: `0` (valid immediately)
- `validBefore`: `Math.floor(Date.now() / 1000) + requiredDeadlineSeconds`
- `nonce`: random 32-byte hex string

#### JavaScript (ethers v6)
```javascript
import { ethers, hexlify, randomBytes } from 'ethers';

async function signPaymentAuthorization(wallet, requirements) {
  const accept = requirements.accepts[0];
  const tokenAddress = "0xYourTokenAddress"; // from config or extra fields
  const chainId = 84532; // base-sepolia

  const domain = {
    name: accept.extra.name,
    version: accept.extra.version,
    chainId,
    verifyingContract: tokenAddress,
  };

  const types = {
    TransferWithAuthorization: [
      { name: "from",        type: "address" },
      { name: "to",          type: "address" },
      { name: "value",       type: "uint256" },
      { name: "validAfter",  type: "uint256" },
      { name: "validBefore", type: "uint256" },
      { name: "nonce",       type: "bytes32" },
    ],
  };

  const amountInTokenUnits = BigInt(1000000); // $1.00 in USDC (6 decimals)
  const validBefore = Math.floor(Date.now() / 1000) + accept.requiredDeadlineSeconds;
  const nonce = hexlify(randomBytes(32));

  const message = {
    from:        wallet.address,
    to:          accept.payToAddress,
    value:       amountInTokenUnits,
    validAfter:  0n,
    validBefore: BigInt(validBefore),
    nonce,
  };

  const signature = await wallet.signTypedData(domain, types, message);

  return { signature, authorization: { ...message, nonce } };
}
```

#### Python (eth_account)
```python
from eth_account import Account
from eth_account.messages import encode_typed_data
import secrets, time

def sign_payment_authorization(private_key: str, requirements: dict, amount_token_units: int) -> dict:
    accept = requirements["accepts"][0]
    valid_before = int(time.time()) + accept["requiredDeadlineSeconds"]
    nonce = "0x" + secrets.token_hex(32)

    domain = {
        "name": accept["extra"]["name"],
        "version": accept["extra"]["version"],
        "chainId": 84532,
        "verifyingContract": "0xYourTokenAddress",
    }
    types = {
        "TransferWithAuthorization": [
            {"name": "from",        "type": "address"},
            {"name": "to",          "type": "address"},
            {"name": "value",       "type": "uint256"},
            {"name": "validAfter",  "type": "uint256"},
            {"name": "validBefore", "type": "uint256"},
            {"name": "nonce",       "type": "bytes32"},
        ]
    }
    message = {
        "from":        Account.from_key(private_key).address,
        "to":          accept["payToAddress"],
        "value":       amount_token_units,
        "validAfter":  0,
        "validBefore": valid_before,
        "nonce":       nonce,
    }

    signed = Account.sign_typed_data(private_key, domain, types, message)
    return {"signature": signed.signature.hex(), "authorization": message}
```

---

## Step 3: Submit Payment

Call `POST /x402/payments` with the signed authorization and the original requirements object.

### Request
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
        "from": "0xAgentAddress",
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

### Response — success
```json
{
  "success": true,
  "transaction": "0xabc123...",
  "network": "base-sepolia"
}
```

### Response — failure
```json
{
  "success": false,
  "error": "invalid signature",
  "network": "base-sepolia"
}
```

### Implementation Examples

#### JavaScript/TypeScript
```javascript
async function submitPayment(requirements, signature, authorization) {
  const accept = requirements.accepts[0];

  const res = await fetch(
    `${ZEROPAY_CONFIG.apiUrl}/x402/payments?apikey=${ZEROPAY_CONFIG.apiKey}`,
    {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        paymentPayload: {
          x402Version: 1,
          scheme: accept.scheme,
          network: accept.network,
          payload: { signature, authorization },
        },
        paymentRequirements: requirements,
      }),
    }
  );

  const result = await res.json();
  if (!result.success) throw new Error(result.error);
  return result; // { success: true, transaction: "0x...", network: "..." }
}
```

#### Python
```python
def submit_payment(requirements: dict, signature: str, authorization: dict) -> dict:
    accept = requirements["accepts"][0]
    res = requests.post(
        f"{ZEROPAY_CONFIG['apiUrl']}/x402/payments",
        params={"apikey": ZEROPAY_CONFIG['apiKey']},
        json={
            "paymentPayload": {
                "x402Version": 1,
                "scheme": accept["scheme"],
                "network": accept["network"],
                "payload": {"signature": signature, "authorization": authorization},
            },
            "paymentRequirements": requirements,
        },
    )
    res.raise_for_status()
    result = res.json()
    if not result["success"]:
        raise ValueError(result["error"])
    return result
```

---

## Complete Payment Flow Example

```javascript
import { ethers } from 'ethers';

async function agentPay(customer, amountCents) {
  // 1. Get requirements
  const requirements = await getPaymentRequirements(customer, amountCents);

  // 2. Sign authorization
  const wallet = new ethers.Wallet(process.env.AGENT_PRIVATE_KEY);
  const { signature, authorization } = await signPaymentAuthorization(
    wallet,
    requirements
  );

  // 3. Submit payment
  const result = await submitPayment(requirements, signature, authorization);

  console.log(`Paid! tx: ${result.transaction}`);
  return result;
}

agentPay("agent_alice", 1000);
```

```python
from eth_account import Account

def agent_pay(customer: str, amount_cents: int) -> dict:
    requirements = get_payment_requirements(customer, amount_cents)
    amount_token = amount_cents * 10000  # cents → USDC 6-decimal units
    auth = sign_payment_authorization(
        private_key=os.environ["AGENT_PRIVATE_KEY"],
        requirements=requirements,
        amount_token_units=amount_token,
    )
    return submit_payment(requirements, auth["signature"], auth["authorization"])

result = agent_pay("agent_alice", 1000)
print(f"Paid! tx: {result['transaction']}")
```

---

## Using the Built-in Rust Client SDK

ZeroPay ships an `x402` crate with a `ClientFacilitator` that handles signing and submission for Rust-based agents:

```rust
use x402::client::{ClientFacilitator, PaymentMethod};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut facilitator = ClientFacilitator::new();
    facilitator.add_payment_method(
        "base-sepolia",
        PaymentMethod::Evm(signer, rpc_url, tokens),
    );

    // Fetch requirements from the server
    let requirements = fetch_requirements(&client, customer, amount).await?;

    // Build, sign, and return the payment payload
    let payload = facilitator.create_payment(&requirements).await?;

    // Submit to ZeroPay
    let response = facilitator.pay(&zeropay_url, payload).await?;

    println!("tx: {}", response.transaction);
    Ok(())
}
```

---

## Discover Available Services

Agents can browse what's available before paying:

```bash
curl "http://localhost:9000/x402/discovery?apikey=your-api-key&limit=10"
```

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

## Check Supported Schemes

```bash
curl "http://localhost:9000/x402/support?apikey=your-api-key"
```

```json
[
  { "scheme": "exact", "network": "base-sepolia", "x402Version": 1 }
]
```

---

## Error Handling

| Scenario | `success` | `error` field |
|----------|-----------|---------------|
| Invalid API key | HTTP 401 | `"user auth error"` |
| Bad request body | HTTP 400 | — |
| Invalid signature | HTTP 200 | `"invalid signature"` |
| Nonce already used | HTTP 200 | `"nonce already used"` |
| Internal error | HTTP 500 | `"internal error"` |

For non-200 HTTP responses:
```json
{ "status": "failure", "error": "user auth error" }
```

```javascript
async function payWithRetry(customer, amount, maxRetries = 3) {
  for (let i = 0; i < maxRetries; i++) {
    const result = await agentPay(customer, amount);
    if (result.success) return result;

    // Nonce collision: generate a new nonce and retry
    if (result.error === "nonce already used") continue;

    // Other errors are not retryable
    throw new Error(result.error);
  }
  throw new Error("Max retries exceeded");
}
```

---

## Security Notes

- **Private keys**: Never log or expose the agent's private key
- **validBefore**: Keep authorization windows short (use `requiredDeadlineSeconds` from requirements)
- **Nonces**: Use cryptographically random 32-byte nonces; never reuse
- **API key**: Store in environment variables, rotate periodically

---

## Summary

| Step | Endpoint | Method | Purpose |
|------|----------|--------|---------|
| Discover | `/x402/requirements` | GET | Get payee address + accepted schemes |
| Pay | `/x402/payments` | POST | Submit signed authorization; settle on-chain |
| Browse | `/x402/discovery` | GET | Find available services |
| Check | `/x402/support` | GET | List supported schemes and networks |

For full API reference see [API.md](../API.md). For protocol details see [x402.md](../x402.md).
