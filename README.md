# ZeroPay
An open-source, self-hosted payment gateway for stablecoins and crypto subscriptions.

## Deploy
1. Build with docker
2. Build from source

## Platform
If you find deployment difficult, you can use the services provided by our platform (we also use it ourselves).
1. open [zeropay.dev](https://zeropay.dev) to register merchant info.
2. use `api.zeropay.dev` as the service

## Payment Session
#### POST `/sessions` create new session for customer to pay
```
request:
{
  customer: string, # the identity customer name
  amount: int, # the amount must with 100 decimal, e.g. $10, use 1000
}

response:
{
  session_id: int,
  customer: string,
  pay_eth: string,
  amount: int,
  expired: DateTime,
  completed: bool,
  *session_url: string, # public payment session url, customer can use it to pay
  *merchant: string,    # merchant name
  *chains: [{           # supported chains
    name: string,       # token name
    address: string,    # token address
  }]
}

TIPS: * is platform-specific and is not required for standalone deployment.
```

#### GET `/sessions/{id}` get the session status
```
response:
same as above create new session response
```

## Webhook

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
