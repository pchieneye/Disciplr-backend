# Webhooks

## Overview

The webhook system delivers lifecycle events (e.g. `vault_created`, `vault_completed`, `vault_failed`, `vault_cancelled`) to registered subscriber URLs via HTTP POST with HMAC-SHA256 signature verification.

## Subscriber Management

Subscribers are stored in-memory (same pattern as API keys). Each subscriber has:

- `id` – UUID
- `url` – target endpoint
- `secret` – HMAC signing key
- `events` – event types to subscribe to (empty = wildcard)
- `active` – delivery flag

### SSRF Protection

`isUrlAllowed()` blocks loopback, link-local, and RFC-1918 addresses. If `WEBHOOK_ALLOWED_HOSTS` is set, the target hostname must also match.

## Delivery

`dispatchWebhookEvent()` sends a payload to all eligible active subscribers. Each delivery is retried with exponential backoff (max 3 attempts).

### Headers

| Header | Description |
|--------|-------------|
| `x-disciplr-signature` | `sha256=<hex-digest>` HMAC-SHA256 of the JSON body |
| `x-disciplr-event` | Event type (e.g. `vault_created`) |
| `x-disciplr-event-id` | Originating event ID in `{txHash}:{eventIndex}` format |
| `x-disciplr-delivery-timestamp` | ISO 8601 timestamp |

## Dead-Letter Queue

When a delivery permanently fails (exhausts retries), the failed delivery is persisted to the `webhook_dead_letters` table for later inspection and replay.

### Schema

| Column | Type | Description |
|--------|------|-------------|
| `id` | UUID | Primary key |
| `subscriber_id` | UUID | Subscriber that failed to receive |
| `event_id` | TEXT | Event ID (`{txHash}:{eventIndex}`) |
| `event_type` | VARCHAR(128) | Event type |
| `payload` | JSONB | Original delivery payload |
| `last_error` | TEXT | Last error message |
| `attempts` | INTEGER | Number of delivery attempts |
| `failed_at` | TIMESTAMPTZ | When the delivery permanently failed |
| `replayed_at` | TIMESTAMPTZ | When the entry was replayed (null if not yet) |

### Admin API

#### GET `/api/admin/webhooks/dead-letters`

List dead-letter entries with optional `subscriber_id` filter.

Query params: `limit`, `offset`, `subscriber_id`

Response:
```json
{
  "webhook_dead_letters": [...],
  "count": 10,
  "total": 42,
  "limit": 50,
  "offset": 0,
  "has_more": true
}
```

#### POST `/api/admin/webhooks/dead-letters/:id/replay`

Replays a dead-letter entry. Validates the URL is still allowed, then re-delivers to the subscriber's in-memory handler. Stamps `replayed_at` on success.

Response (202):
```json
{ "replayed": true }
```

Response (404):
```json
{ "error": "Dead letter not found or already replayed" }
```

## Testing

Run webhook tests:
```bash
npm test -- --testPathPattern=webhooks
```

DLQ tests require a PostgreSQL database (`DATABASE_URL`). Without it, they are skipped gracefully.

---

# Webhook Delivery System

The webhook system delivers vault lifecycle events to registered HTTP endpoints. Subscribers are stored in PostgreSQL and scoped per organization.

## Storage Model

Webhook subscribers are stored in the `webhook_subscribers` table:

| Column | Type | Description |
|---|---|---|
| `id` | `uuid` (PK, auto-generated) | Unique subscriber identifier |
| `organization_id` | `varchar(255)` | Owning organization (NOT NULL) |
| `url` | `varchar(2048)` | Target webhook URL |
| `secret` | `text` | HMAC signing secret |
| `events` | `jsonb` | Array of event types to receive; empty array = wildcard (all events) |
| `active` | `boolean` | Whether the subscriber is active |
| `created_at` | `timestamptz` | Creation timestamp |
| `updated_at` | `timestamptz` | Last update timestamp |

Index: `(organization_id, active)` for efficient org-scoped lookups.

## Secret Handling Decision

The `secret` column stores the HMAC signing secret in plaintext. Hashing is not viable because the raw secret is required to compute HMAC-SHA256 signatures for outgoing webhook requests.

**Recommendation for production:** Encrypt the secret at rest using one of:
- PostgreSQL `pgcrypto` extension (`pgp_sym_encrypt` / `pgp_sym_decrypt`)
- Application-level AES-256-GCM encryption before storage, with the encryption key managed via a secrets manager (AWS KMS, HashiCorp Vault)

The trade-off is that the encryption key must be available to the application at runtime to decrypt secrets for signing, which shifts the protection boundary from the database layer to the key management layer.

## Organization Isolation

All subscriber queries are scoped by `organization_id`. When dispatching events, only subscribers belonging to the same organization as the event source receive the delivery. This prevents cross-tenant information leakage.

## API

### `addSubscriber(organizationId, url, secret, events)`

Creates a new webhook subscriber. The URL is validated against the SSRF allowlist (`isUrlAllowed`). Returns the created subscriber.

### `removeSubscriber(id)`

Deletes a subscriber by ID. Returns `true` if found.

### `listSubscribers(organizationId)`

Returns all active subscribers for an organization.

### `dispatchWebhookEvent(payload)`

Delivers an event to all eligible active subscribers for the organization specified in `payload.organizationId`. Uses exponential-backoff retry (max 3 attempts). Failures are collected per-subscriber.
