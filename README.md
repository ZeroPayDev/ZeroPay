# ZeroPay
An open-source, self-hosted payment gateway for stablecoins and crypto subscriptions.

## Deploy
1. Build with docker
2. Build from source

## Platform
Enjoy a simpler and more stable platform version. Please note that the platform will deduct part of the commission rate as a handling gas fee.
1. open [zeropay.dev](https://zeropay.dev) to register merchant info.
2. use `https://api.zeropay.dev` as the service

## Payment API
#### POST `/sessions?apikey=xxxx` create new session for customer to pay
```
request:
{
  customer: string, # the identity customer name
  amount: int,      # the amount must with 100 decimal, e.g. $10, use 1000
}

response:
{
  session_id: int,          # session id
  customer: string,         # customer name
  pay_eth: string,          # eth-like chains payment address
  amount: int,              # the amount of this session
  expired: DateTime,        # the session expired time
  completed: bool,          # the session is completed or not
  *session_url: string,     # public payment session url, customer can use it to pay
  *merchant: string,        # merchant name
  *chains: [                # supported chains
    {
      name: string,         # chain name
      estimation: int,      # estimated time to receive payment (seconds)
      commission: int,      # commission rate 1-100(100%)
      commission_min: int,  # commission minimum, e.g. 10($0.1)
      commission_max: int,  # commission maximum, e.g. 100($1)
      tokens: {
        "USDT": {           # supported token
          identity: string, # token identity
          name: string,     # token name
          address: string,  # token address
        },
        "USDC": {           # supported token
          identity: string, # token identity
          name: string,     # token name
          address: string,  # token address
        },
      }
    },
    {
      ...
    }
  ]
}

TIPS: * is platform-specific and is not required for standalone deployment.
```

#### GET `/sessions/{id}?apikey=xxxx` get the session status
```
response:
same as above create new session response
```

## Payment Webhook
all webhook event request will use `HMAC`, the hmac use the `sha256` hash(sha2_256).
1. use `apikey` as the secret key
2. the header `X-HMAC` is the code
3. the request body is the message

### Events
#### `session.paid` when customer paid the money
```
{
  event: "session.paid",
  params: [1, "neo", 1000], # session id, customer name, customer deposit amount (with 100 decimal, here is $10.00)
}
```

#### `session.settled` when money (without commission) sent to merchant account
```
{
  event: "session.settled",
  params: [1, "neo", 9500], # session id, customer name, settled amount
}
```

#### `unknow.paid` when received money, but no session linked to it
```
{
  event: "unknow.paid",
  params: ["neo", 1000], # customer name, customer deposit amount
}
```

#### `unknow.settled` when then unknow money sent to merchant account
```
{
  event: "unknow.settled",
  params: ["neo", 9500], # customer name, settled amount
}
```

## License

This project is licensed under [GPLv3](https://www.gnu.org/licenses/gpl-3.0.en.html).
