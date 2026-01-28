# gRPC API

Auth9 提供 gRPC 服务用于高性能的服务间通信，主要用于 Token 交换和权限验证。

## 概述

- **协议**: gRPC (HTTP/2)
- **序列化**: Protocol Buffers (proto3)
- **默认端口**: 50051
- **TLS**: 生产环境强制启用

## Proto 定义

### TokenExchange Service

```protobuf
syntax = "proto3";

package auth9.v1;

// Token 交换服务
service TokenExchange {
  // 交换租户访问令牌
  rpc ExchangeToken(ExchangeTokenRequest) returns (ExchangeTokenResponse);
  
  // 验证令牌
  rpc ValidateToken(ValidateTokenRequest) returns (ValidateTokenResponse);
  
  // 获取用户角色
  rpc GetUserRoles(GetUserRolesRequest) returns (GetUserRolesResponse);
  
  // Token 内省
  rpc IntrospectToken(IntrospectTokenRequest) returns (IntrospectTokenResponse);
}

// 交换 Token 请求
message ExchangeTokenRequest {
  string identity_token = 1;        // 用户身份令牌
  string tenant_id = 2;             // 目标租户 ID
  string service_client_id = 3;     // 服务客户端 ID
  repeated string scopes = 4;       // 请求的权限范围
}

// 交换 Token 响应
message ExchangeTokenResponse {
  string access_token = 1;          // 租户访问令牌
  string token_type = 2;            // Token 类型（Bearer）
  int32 expires_in = 3;             // 过期时间（秒）
  repeated string roles = 4;        // 用户角色列表
  repeated string permissions = 5;  // 用户权限列表
}

// 验证 Token 请求
message ValidateTokenRequest {
  string token = 1;                 // 待验证的 Token
  string expected_audience = 2;     // 期望的受众
}

// 验证 Token 响应
message ValidateTokenResponse {
  bool valid = 1;                   // 是否有效
  string user_id = 2;               // 用户 ID
  string tenant_id = 3;             // 租户 ID
  repeated string roles = 4;        // 角色列表
  int64 expires_at = 5;             // 过期时间戳
  string error_message = 6;         // 错误信息
}

// 获取用户角色请求
message GetUserRolesRequest {
  string user_id = 1;               // 用户 ID
  string tenant_id = 2;             // 租户 ID
  string service_id = 3;            // 服务 ID（可选）
}

// 获取用户角色响应
message GetUserRolesResponse {
  repeated Role roles = 1;          // 角色列表
  repeated string permissions = 2;  // 权限列表
}

// 角色信息
message Role {
  string id = 1;                    // 角色 ID
  string name = 2;                  // 角色名称
  string display_name = 3;          // 显示名称
  string service_id = 4;            // 所属服务 ID
  string service_name = 5;          // 服务名称
}

// Token 内省请求
message IntrospectTokenRequest {
  string token = 1;                 // Token
  string token_type_hint = 2;       // Token 类型提示
}

// Token 内省响应
message IntrospectTokenResponse {
  bool active = 1;                  // 是否活跃
  string scope = 2;                 // 权限范围
  string client_id = 3;             // 客户端 ID
  string username = 4;              // 用户名
  string token_type = 5;            // Token 类型
  int64 exp = 6;                    // 过期时间
  int64 iat = 7;                    // 签发时间
  string sub = 8;                   // 主体
  string aud = 9;                   // 受众
  string iss = 10;                  // 签发者
  string jti = 11;                  // Token ID
}
```

## 使用示例

### Rust 客户端

#### 1. 添加依赖

```toml
[dependencies]
tonic = "0.11"
prost = "0.12"
tokio = { version = "1", features = ["full"] }

[build-dependencies]
tonic-build = "0.11"
```

#### 2. 构建客户端

```rust
use tonic::Request;
use auth9::token_exchange_client::TokenExchangeClient;
use auth9::{ExchangeTokenRequest, ExchangeTokenResponse};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 连接到 gRPC 服务
    let mut client = TokenExchangeClient::connect("http://auth9-core:50051").await?;

    // 创建请求
    let request = Request::new(ExchangeTokenRequest {
        identity_token: "user-identity-token".to_string(),
        tenant_id: "tenant-uuid".to_string(),
        service_client_id: "my-service-client-id".to_string(),
        scopes: vec!["read".to_string(), "write".to_string()],
    });

    // 调用服务
    let response = client.exchange_token(request).await?;
    let token_response = response.into_inner();

    println!("Access Token: {}", token_response.access_token);
    println!("Roles: {:?}", token_response.roles);
    println!("Permissions: {:?}", token_response.permissions);

    Ok(())
}
```

#### 3. 验证 Token

```rust
use auth9::{ValidateTokenRequest, ValidateTokenResponse};

async fn validate_token(
    client: &mut TokenExchangeClient<tonic::transport::Channel>,
    token: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let request = Request::new(ValidateTokenRequest {
        token: token.to_string(),
        expected_audience: "my-service".to_string(),
    });

    let response = client.validate_token(request).await?;
    let validation = response.into_inner();

    if validation.valid {
        println!("Token 有效");
        println!("用户 ID: {}", validation.user_id);
        println!("租户 ID: {}", validation.tenant_id);
        println!("角色: {:?}", validation.roles);
        Ok(true)
    } else {
        println!("Token 无效: {}", validation.error_message);
        Ok(false)
    }
}
```

#### 4. 获取用户角色

```rust
use auth9::{GetUserRolesRequest, GetUserRolesResponse};

async fn get_user_roles(
    client: &mut TokenExchangeClient<tonic::transport::Channel>,
    user_id: &str,
    tenant_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let request = Request::new(GetUserRolesRequest {
        user_id: user_id.to_string(),
        tenant_id: tenant_id.to_string(),
        service_id: "".to_string(), // 可选
    });

    let response = client.get_user_roles(request).await?;
    let roles_response = response.into_inner();

    println!("用户角色:");
    for role in roles_response.roles {
        println!("  - {} ({})", role.display_name, role.name);
    }

    println!("用户权限:");
    for permission in roles_response.permissions {
        println!("  - {}", permission);
    }

    Ok(())
}
```

### Go 客户端

#### 1. 安装依赖

```bash
go get google.golang.org/grpc
go get google.golang.org/protobuf
```

#### 2. 使用示例

```go
package main

import (
    "context"
    "fmt"
    "log"
    "time"

    "google.golang.org/grpc"
    "google.golang.org/grpc/credentials/insecure"
    pb "your-module/auth9/v1"
)

func main() {
    // 连接到 gRPC 服务
    conn, err := grpc.Dial(
        "auth9-core:50051",
        grpc.WithTransportCredentials(insecure.NewCredentials()),
    )
    if err != nil {
        log.Fatalf("连接失败: %v", err)
    }
    defer conn.Close()

    client := pb.NewTokenExchangeClient(conn)

    // 交换 Token
    ctx, cancel := context.WithTimeout(context.Background(), time.Second*10)
    defer cancel()

    resp, err := client.ExchangeToken(ctx, &pb.ExchangeTokenRequest{
        IdentityToken:   "user-identity-token",
        TenantId:        "tenant-uuid",
        ServiceClientId: "my-service-client-id",
        Scopes:          []string{"read", "write"},
    })
    if err != nil {
        log.Fatalf("Token 交换失败: %v", err)
    }

    fmt.Printf("Access Token: %s\n", resp.AccessToken)
    fmt.Printf("Roles: %v\n", resp.Roles)
    fmt.Printf("Permissions: %v\n", resp.Permissions)
}
```

### Node.js 客户端

#### 1. 安装依赖

```bash
npm install @grpc/grpc-js @grpc/proto-loader
```

#### 2. 使用示例

```javascript
const grpc = require('@grpc/grpc-js');
const protoLoader = require('@grpc/proto-loader');

// 加载 proto 文件
const packageDefinition = protoLoader.loadSync(
  'path/to/token_exchange.proto',
  {
    keepCase: true,
    longs: String,
    enums: String,
    defaults: true,
    oneofs: true
  }
);

const auth9Proto = grpc.loadPackageDefinition(packageDefinition).auth9.v1;

// 创建客户端
const client = new auth9Proto.TokenExchange(
  'auth9-core:50051',
  grpc.credentials.createInsecure()
);

// 交换 Token
client.exchangeToken({
  identity_token: 'user-identity-token',
  tenant_id: 'tenant-uuid',
  service_client_id: 'my-service-client-id',
  scopes: ['read', 'write']
}, (error, response) => {
  if (error) {
    console.error('错误:', error);
    return;
  }
  
  console.log('Access Token:', response.access_token);
  console.log('Roles:', response.roles);
  console.log('Permissions:', response.permissions);
});

// 验证 Token
client.validateToken({
  token: 'access-token-to-validate',
  expected_audience: 'my-service'
}, (error, response) => {
  if (error) {
    console.error('错误:', error);
    return;
  }
  
  if (response.valid) {
    console.log('Token 有效');
    console.log('用户 ID:', response.user_id);
    console.log('租户 ID:', response.tenant_id);
  } else {
    console.log('Token 无效:', response.error_message);
  }
});
```

### Python 客户端

#### 1. 安装依赖

```bash
pip install grpcio grpcio-tools
```

#### 2. 生成代码

```bash
python -m grpc_tools.protoc \
  -I./proto \
  --python_out=. \
  --grpc_python_out=. \
  ./proto/token_exchange.proto
```

#### 3. 使用示例

```python
import grpc
import auth9_pb2
import auth9_pb2_grpc

# 创建连接
channel = grpc.insecure_channel('auth9-core:50051')
stub = auth9_pb2_grpc.TokenExchangeStub(channel)

# 交换 Token
request = auth9_pb2.ExchangeTokenRequest(
    identity_token='user-identity-token',
    tenant_id='tenant-uuid',
    service_client_id='my-service-client-id',
    scopes=['read', 'write']
)

response = stub.ExchangeToken(request)
print(f"Access Token: {response.access_token}")
print(f"Roles: {response.roles}")
print(f"Permissions: {response.permissions}")

# 验证 Token
validate_request = auth9_pb2.ValidateTokenRequest(
    token='access-token-to-validate',
    expected_audience='my-service'
)

validate_response = stub.ValidateToken(validate_request)
if validate_response.valid:
    print("Token 有效")
    print(f"用户 ID: {validate_response.user_id}")
    print(f"租户 ID: {validate_response.tenant_id}")
else:
    print(f"Token 无效: {validate_response.error_message}")
```

## 性能优化

### 连接池

使用连接池提高性能：

```rust
use tonic::transport::Channel;
use tower::ServiceBuilder;
use std::time::Duration;

async fn create_channel() -> Result<Channel, Box<dyn std::error::Error>> {
    let channel = Channel::from_static("http://auth9-core:50051")
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(10))
        .tcp_keepalive(Some(Duration::from_secs(60)))
        .http2_keep_alive_interval(Duration::from_secs(30))
        .connect()
        .await?;
    
    Ok(channel)
}
```

### 超时控制

```rust
use tonic::Request;
use std::time::Duration;

let mut request = Request::new(ExchangeTokenRequest {
    // ...
});

// 设置超时
request.set_timeout(Duration::from_millis(100));

let response = client.exchange_token(request).await?;
```

### 元数据传递

```rust
use tonic::metadata::MetadataValue;

let mut request = Request::new(ExchangeTokenRequest {
    // ...
});

// 添加元数据
request.metadata_mut().insert(
    "authorization",
    MetadataValue::from_str("Bearer token")?,
);

request.metadata_mut().insert(
    "x-tenant-id",
    MetadataValue::from_str("tenant-uuid")?,
);
```

## TLS 配置

### 服务端 TLS

```rust
use tonic::transport::{Server, ServerTlsConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cert = tokio::fs::read("server.crt").await?;
    let key = tokio::fs::read("server.key").await?;
    
    let tls_config = ServerTlsConfig::new()
        .identity(tonic::transport::Identity::from_pem(cert, key));

    let addr = "0.0.0.0:50051".parse()?;
    
    Server::builder()
        .tls_config(tls_config)?
        .add_service(TokenExchangeServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
```

### 客户端 TLS

```rust
use tonic::transport::{Channel, ClientTlsConfig};

async fn create_secure_channel() -> Result<Channel, Box<dyn std::error::Error>> {
    let ca_cert = tokio::fs::read("ca.crt").await?;
    
    let tls_config = ClientTlsConfig::new()
        .ca_certificate(tonic::transport::Certificate::from_pem(ca_cert))
        .domain_name("auth9.yourdomain.com");

    let channel = Channel::from_static("https://auth9-core:50051")
        .tls_config(tls_config)?
        .connect()
        .await?;
    
    Ok(channel)
}
```

## 错误处理

gRPC 状态码：

| 状态码 | 说明 |
|-------|------|
| `OK` | 成功 |
| `CANCELLED` | 操作被取消 |
| `INVALID_ARGUMENT` | 参数无效 |
| `DEADLINE_EXCEEDED` | 超时 |
| `NOT_FOUND` | 资源不存在 |
| `ALREADY_EXISTS` | 资源已存在 |
| `PERMISSION_DENIED` | 权限被拒绝 |
| `UNAUTHENTICATED` | 未认证 |
| `RESOURCE_EXHAUSTED` | 资源耗尽 |
| `FAILED_PRECONDITION` | 前置条件失败 |
| `INTERNAL` | 内部错误 |
| `UNAVAILABLE` | 服务不可用 |

## 监控指标

gRPC 服务暴露以下指标：

- `grpc_server_started_total` - 请求总数
- `grpc_server_handled_total` - 处理完成总数
- `grpc_server_msg_received_total` - 接收消息数
- `grpc_server_msg_sent_total` - 发送消息数
- `grpc_server_handling_seconds` - 请求处理时长

## 相关文档

- [REST API](REST-API.md)
- [Token 规范](Token规范.md)
- [认证流程](认证流程.md)
- [性能优化](性能优化.md)
