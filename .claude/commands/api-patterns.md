# API Patterns

> API design principles for the Cimmeria admin API. Apply when designing or reviewing REST endpoints.

---

## Decision Checklist

Before designing an API endpoint:

- [ ] Who are the API consumers? (frontend admin panel, CLI tools, other services)
- [ ] Chosen appropriate HTTP method and status codes?
- [ ] Defined consistent response format?
- [ ] Considered authentication/authorization needs?
- [ ] Rate limiting needed?
- [ ] Documentation updated?

## REST Design Rules

### Resource Naming

```
GET    /api/players          # List players
GET    /api/players/:id      # Get player
POST   /api/players          # Create player
PUT    /api/players/:id      # Update player
DELETE /api/players/:id      # Delete player

GET    /api/players/:id/inventory    # Nested resource
GET    /api/spaces/:id/entities      # Entities in a space
```

### HTTP Methods

| Method | Use | Idempotent |
|--------|-----|------------|
| GET | Read resource(s) | Yes |
| POST | Create resource | No |
| PUT | Replace resource | Yes |
| PATCH | Partial update | Yes |
| DELETE | Remove resource | Yes |

### Status Codes

| Code | Use |
|------|-----|
| 200 | Success (with body) |
| 201 | Created (POST success) |
| 204 | No Content (DELETE success) |
| 400 | Bad Request (validation error) |
| 401 | Unauthorized (not authenticated) |
| 403 | Forbidden (insufficient access level) |
| 404 | Not Found |
| 409 | Conflict (duplicate resource) |
| 422 | Unprocessable Entity (semantic error) |
| 500 | Internal Server Error |

## Response Envelope

### Success

```json
{
  "data": { ... },
  "meta": {
    "total": 100,
    "page": 1,
    "per_page": 25
  }
}
```

### Error

```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Player name must be 3-20 characters",
    "details": [
      { "field": "name", "message": "Too short" }
    ]
  }
}
```

## Pagination

```
GET /api/players?page=2&per_page=25&sort=level&order=desc
```

Response includes:
```json
{
  "data": [...],
  "meta": {
    "total": 342,
    "page": 2,
    "per_page": 25,
    "total_pages": 14
  }
}
```

## Filtering & Search

```
GET /api/players?search=tealc&archetype=soldier&min_level=10
GET /api/missions?status=active&zone=abydos
GET /api/logs?level=error&since=2026-03-01
```

## Authentication

This project uses session-based auth:
- Login via `/api/login` with account credentials
- Session stored in cookie
- Access level >= 1 required for admin panel
- Check `accesslevel` for admin tiers

## WebSocket Patterns

For real-time data (logs, entity updates, chat):

```typescript
// Event stream pattern
ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  switch (msg.type) {
    case 'log': handleLog(msg.payload); break;
    case 'entity_update': handleEntity(msg.payload); break;
    case 'chat': handleChat(msg.payload); break;
  }
};
```

## Anti-Patterns

| Don't | Do |
|-------|-----|
| Verbs in endpoints (`/getPlayers`) | Nouns (`/players`) |
| Inconsistent response formats | Standardized envelope |
| Expose internal errors to clients | Generic error + log details server-side |
| Return everything always | Support field selection or pagination |
| Skip validation | Validate at API boundary |
| Ignore access levels | Check authorization per endpoint |

## Project API Reference

Existing endpoints in `crates/admin-api/src/routes/`:
- `auth.rs` — login/logout/me
- `config.rs` — server configuration
- `content.rs` — content engine (chains, missions)
- `entities.rs` — entity queries
- `players.rs` — player/character management
- `spaces.rs` — world space data

WebSocket streams in `crates/admin-api/src/ws/`:
- `entity_stream.rs` — live entity updates
- `event_stream.rs` — server events
- `log_stream.rs` — log streaming
