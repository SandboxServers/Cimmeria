# Login & Authentication Flow

Complete login sequence for Stargate Worlds, reconstructed from strings and RTTI in sgw.exe.

## Overview

SGW uses a two-phase SOAP-based login:
1. **Phase 1: Authentication** — Client authenticates via `/SGWLogin/UserAuth`
2. **Phase 2: Server Selection** — Client selects a shard via `/SGWLogin/ServerSelection`
3. **Phase 3: BaseApp Connection** — Client connects directly to assigned BaseApp via Mercury

## SOAP Endpoints

### Phase 1: User Authentication
| Property | Value | Address |
|----------|-------|---------|
| **Endpoint** | `/SGWLogin/UserAuth` | 0x019cec40 |
| **Request Type** | `sgwLogin:SGWLoginRequest` | 0x019cec24 |
| **Namespace** | `http://www.stargateworlds.com/xml/sgwlogin` | 0x019ceb8c |
| **Namespace Prefix** | `sgwLogin` | 0x019ceb80 |

### Phase 2: Server Selection
| Property | Value | Address |
|----------|-------|---------|
| **Endpoint** | `/SGWLogin/ServerSelection` | 0x019cf1dc |
| **Request Type** | `sgwLogin:SGWSelectServerRequest` | 0x019cf1bc |

## XML Schema Types

All types in the `sgwLogin` namespace. Addresses show first occurrence in .rdata.

### Primitive Types
| Type | Address | Purpose |
|------|---------|---------|
| `sgwLogin:LoadType` | 0x01b24ddc | Server load indicator |
| `sgwLogin:SessionKeyType` | 0x01b24df0 | 64-byte hex session key |
| `sgwLogin:ServerNameType` | 0x01b24e78 | Shard/server name |
| `sgwLogin:BaseAppAddress` | 0x01b24e90 | IP:port of BaseApp |
| `sgwLogin:ServerLocationType` | 0x01b24ea8 | Server location descriptor |
| `sgwLogin:TicketType` | 0x01b24ec4 | Authentication ticket |
| `sgwLogin:ProtocolDigest` | 0x01b2507c | Protocol version hash |

### Response Types
| Type | Address | Purpose |
|------|---------|---------|
| `sgwLogin:SGWLoginResponse` | 0x01b255bc | Base login response |
| `sgwLogin:SGWLoginResponse-SGWLoginSuccess` | 0x01b24f6c | Successful login |
| `sgwLogin:SGWLoginResponse-SGWLoginSuccess-SGWShardListResp` | 0x01b25458 | Shard list |
| `sgwLogin:SGWLoginResponse-SGWLoginSuccess-AccountInfo` | 0x01b25494 | Account details |
| `sgwLogin:SGWServerLocationResponse` | 0x01b25578 | Server selection response |
| `sgwLogin:UserPendingBaseAppMgrRequest` | 0x01b25550 | Pending BaseApp allocation |
| `sgwLogin:UserPendingBaseAppMgrResponse` | 0x01b25528 | BaseApp allocation result |

### RTTI Classes (C++ Implementation)
| Class | RTTI Address |
|-------|-------------|
| `_sgwLogin__SGWLoginRequest@SGWLogin` | 0x01e51ef0 |
| `_sgwLogin__SGWSelectServerRequest@SGWLogin` | 0x01e51e88 |
| `ga__GlobalAuthReq@SGWLogin` | 0x01e51ec4 |
| `sgwLogin__TicketType@SGWLogin` | 0x01e9345c |
| `sgwLogin__ServerLocationType@SGWLogin` | 0x01e93488 |
| `sgwLogin__BaseAppAddress@SGWLogin` | 0x01e934bc |
| `sgwLogin__ServerNameType@SGWLogin` | 0x01e934ec |
| `_sgwLogin__SGWLoginResponse@SGWLogin` | 0x01e935d4 |
| `_sgwLogin__SGWLoginResponse_SGWLoginSuccess@SGWLogin` | 0x01e93828 |
| `_sgwLogin__SGWLoginResponse_SGWLoginSuccess_SGWShardListResp@SGWLogin` | 0x01e93580 |
| `_sgwLogin__SGWLoginResponse_SGWLoginSuccess_AccountInfo@SGWLogin` | 0x01e937d8 |
| `_sgwLogin__SGWServerLocationResponse@SGWLogin` | 0x01e93608 |
| `_sgwLogin__UserPendingBaseAppMgrRequest@SGWLogin` | 0x01e93644 |
| `_sgwLogin__UserPendingBaseAppMgrResponse@SGWLogin` | 0x01e93688 |
| `cmebase__ErrorBase@SGWLogin` | 0x01e936c8 |
| `cmebase__NVP@SGWLogin` | 0x01e936f4 |
| `ga__GlobalAuthRes@SGWLogin` | 0x01e9374c |
| `_ga__GlobalAuthResponse@SGWLogin` | 0x01e93778 |
| `_ga__GlobalAuthRequest@SGWLogin` | 0x01e937a8 |
| `xsd__hexBinary@SGWLogin` | 0x01e93434 |
| `cmehwprofile__Device@SGWLogin` | 0x01e9386c |
| `cmehwprofile__Config@SGWLogin` | 0x01e93898 |

## Login Error Codes

31 error codes extracted from the binary. Address range: 0x019aaf34 - 0x019ab7b8.

### Connection Errors
| Error Code | Address | Description |
|------------|---------|-------------|
| `LoginMessage_LoggedOn` | 0x019aaf34 | Successfully logged in |
| `LoginMessage_ConnectionFailed` | 0x019aaf60 | TCP/UDP connection failed |
| `LoginMessage_DNSLookupFailed` | 0x019aaf9c | DNS resolution failed |
| `LoginMessage_Cancelled` | 0x019ab00c | User cancelled login |
| `LoginMessage_AlreadyOnlineLocally` | 0x019ab040 | Another client instance running |

### Authentication Errors
| Error Code | Address | Description |
|------------|---------|-------------|
| `LoginMessage_NoSuchUser` | 0x019ab084 | Username not found |
| `LoginMessage_InvalidPassword` | 0x019ab0b4 | Wrong password |
| `LoginMessage_UserAlreadyLoggedIn` | 0x019ab0f0 | Account already in use |
| `LoginMessage_IllegalChars` | 0x019ab1f0 | Invalid characters in credentials |

### Server-Side Errors
| Error Code | Address | Description |
|------------|---------|-------------|
| `LoginMessage_DefsDigestMismatch` | 0x019ab138 | Client/server def version mismatch |
| `LoginMessage_DatabaseRejection` | 0x019ab178 | DB rejected login |
| `LoginMessage_ServerDatabase` | 0x019ab1b8 | Database error |
| `LoginMessage_UnknownError` | 0x019aafd8 | Unclassified error |
| `LoginMessage_UnknownFailure` | 0x019ab224 | Unclassified failure |

### Protocol-Level Rejections (LoginApp → Client)
| Error Code | Address | Description |
|------------|---------|-------------|
| `LoginMessage_LoginMalformedRequest` | 0x019ab268 | Bad request format |
| `LoginMessage_LoginBadProtocolVersion` | 0x019ab2b0 | Protocol version mismatch |
| `LoginMessage_LoginRejectedNoSuchUser` | 0x019ab300 | User not found (server-side) |
| `LoginMessage_LoginRejectedInvalidPassword` | 0x019ab350 | Bad password (server-side) |
| `LoginMessage_LoginRejectedAlreadyLoggedIn` | 0x019ab3b0 | Already logged in (server-side) |
| `LoginMessage_LoginRejectedBadDigest` | 0x019ab408 | Protocol digest mismatch |
| `LoginMessage_LoginRejectedDbGeneralFailure` | 0x019ab450 | DB general failure |
| `LoginMessage_LoginRejectedDbNotReady` | 0x019ab4a8 | DB not ready |
| `LoginMessage_LoginRejectedIllegalCharacters` | 0x019ab4f8 | Illegal characters (server) |

### Infrastructure Errors
| Error Code | Address | Description |
|------------|---------|-------------|
| `LoginMessage_LoginRejectedBaseappmgrNotReady` | 0x019ab550 | BaseAppMgr not initialized |
| `LoginMessage_LoginRejectedUpdaterNotReady` | 0x019ab5b0 | Updater service not ready |
| `LoginMessage_LoginRejectedNoBaseapps` | 0x019ab608 | No BaseApp instances available |
| `LoginMessage_LoginRejectedBaseappOverload` | 0x019ab658 | BaseApp at capacity |
| `LoginMessage_LoginRejectedCellappOverload` | 0x019ab6b0 | CellApp at capacity |
| `LoginMessage_LoginRejectedBaseappTimeout` | 0x019ab708 | BaseApp connection timeout |
| `LoginMessage_LoginRejectedBaseappmgrTimeout` | 0x019ab760 | BaseAppMgr response timeout |
| `LoginMessage_LoginRejectedDbmgrOverload` | 0x019ab7b8 | DBMgr at capacity |

## Session Key Exchange

After successful login, the server responds with a `ServerLocation` containing a `SessionKey`:
- Key format: 64-byte hexadecimal string
- Extracted from XML: `<ServerLocation SessionKey="...">` at offset 0x1C in the response
- Used for AES session encryption between client and BaseApp
- Key is logged to `sessions\YYYY-MM-DD_HH-MM-keys.txt` by the AtreaRL sniffer

## Connection Sequence (Reconstructed)

```
Client                    LoginApp              BaseAppMgr           BaseApp
  |                          |                      |                   |
  |-- SGWLoginRequest ------>|                      |                   |
  |   (username, password,   |                      |                   |
  |    protocol digest)      |                      |                   |
  |                          |                      |                   |
  |<-- SGWLoginResponse -----|                      |                   |
  |   (SGWLoginSuccess:      |                      |                   |
  |    ShardList, AcctInfo)  |                      |                   |
  |                          |                      |                   |
  |-- SGWSelectServerReq --->|                      |                   |
  |   (server name)          |                      |                   |
  |                          |-- UserPending ------>|                   |
  |                          |   BaseAppMgrReq      |                   |
  |                          |                      |                   |
  |                          |<- UserPending -------|                   |
  |                          |   BaseAppMgrResp     |                   |
  |                          |   (BaseAppAddress)   |                   |
  |                          |                      |                   |
  |<-- SGWServerLocation ----|                      |                   |
  |   Response (SessionKey,  |                      |                   |
  |    BaseApp IP:port)      |                      |                   |
  |                          |                      |                   |
  |-- baseAppLogin (Mercury) ----------------------------------------->|
  |   (ticket, session key)  |                      |                   |
  |                          |                      |                   |
  |<-- Entity creation, ----------------------------------------------|
  |    world setup           |                      |                   |
```

## BaseApp Connection

After receiving the BaseApp address, the client connects via Mercury:

| String | Address | Purpose |
|--------|---------|---------|
| `BaseAppExtInterface` | 0x019d086c | Interface name for BaseApp external API |
| `baseAppLogin` | 0x019d0880 | Login RPC name |
| `restoreBaseApp` | 0x019d0f50 | Reconnection/restore RPC |
| `BaseAppLoginHandler::BaseAppLoginHandler: calling Nub::send` | 0x019d1a5c | Debug log during login |

### Timeout Handling
```
LoginReplyHandler::onBaseAppReply(): A baseAppLogin message timed out, %d attempts pending
```
Address: 0x019cf038

```
Unable to connect to BaseApp: The client timed out waiting for a response to its connection attempt.
```
Address: 0x019cf0c8

### Address Validation
```
ServerConnection::logOnComplete: BaseApp address on login reply (%s) differs from winning BaseApp reply (%s)
```
Address: 0x019cf388 — Validates the BaseApp address matches across login phases.
