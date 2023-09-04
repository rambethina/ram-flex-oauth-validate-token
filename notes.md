# Notes

## What is the mod key word

## Where are the libraries stored

## How to install dependencies

```cmd
make build
```

## What do enum mean and the following

```rust
pub enum FilterError {
    Unexpected,
    NoToken,
    InactiveToken,
    ExpiredToken,
    NotYetActive,
    ClientError(HttpClientError), //What does this mean
    NonParsableIntrospectionBody(serde_json::Error),
}
```

## What does the following achieve

```rust
#[derive(Deserialize)]
```

## Option<u64> Why not what the system supports

## Result<IntrospectionResponse, FilterError>

## Code below

```rust
    let body = serde_urlencoded::to_string(
        [("token", token)]
    ).map_err(|_| FilterError::Unexpected)?;
```

## map error statement

```rust
.map_err(FilterError::ClientError)?;
```

## Flex read up on , & where is documentation available.

```rust
config.token_extractor
```

```rust
resolve_on_headers(&request)
```

## Following statement

```rust
    //validates if token has expired
    if response.exp.map( |exp | now > exp).unwrap_or_default() {
        return Err(FilterError::ExpiredToken)
    }
```

## Following statement

```rust
Ok(())
```

## where do we get request & client from the following

```rust
    let filter = on_request(
        |request, client| request_filter(request, client, &config))
    );
```

## What additional data can be passed to configure method

```rust
async fn configure(
    launcher: Launcher,
    Configuration(bytes): Configuration,
    cache_builder: CacheBuilder,
) -> Result<()>
```
