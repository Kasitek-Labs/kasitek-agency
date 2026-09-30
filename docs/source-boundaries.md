# Agency source boundaries

This note records the source-only relocation covered by [Agency #2](https://github.com/Kasitek-Labs/kasitek-agency/issues/2)
and [Agency #3](https://github.com/Kasitek-Labs/kasitek-agency/issues/3), with the later Growth
cleanup tracked in [Growth #8](https://github.com/Kasitek-Labs/kasitek-growth/issues/8). The source
was read from Growth's tracked `growth/main` at `0ae32bf350a074cf3c643dcf221e47f53ddb4f63`.
Growth's active worktree was left untouched.

## Owned source

- `server/kasitek-tenants/` contains the tenant API, SQL schema changes, and migrations.
- `client/apps/tenant-portal/` contains the portal source and its authored `public/` assets.
- `client/packages/api-client-react/` is the one internal frontend package imported by the
  portal. It makes same-origin requests under `/api/tenant`.

The Growth portal's tracked `client/apps/tenant-portal/dist/` files were not copied. Agency #3
authorizes the portal source and assets but does not authorize generated build output. The authored
favicon and Open Graph image under `public/` were copied.

## Shared Rust dependencies

The tenant service directly references these shared crates at `server/crates/` in Growth:

| Crate | Original Growth source path | Tenant API manifest edge |
| --- | --- | --- |
| `analytics-producer` | `server/crates/analytics-producer` | private Git dependency from Labs |
| `analytics-reporting` | `server/crates/analytics-reporting` | private Git dependency from Labs |
| `config` | `server/crates/config` | private Git dependency for generic helpers only |
| `crypto-utils` | `server/crates/crypto-utils` | private Git dependency from Labs |
| `observability` | `server/crates/observability` | private Git dependency from Labs |

The Tenant API's `TenantServerConfig` and `CookieSameSitePolicy` now live in
`server/kasitek-tenants/src/config.rs`; the service continues to depend on the existing `config`
crate only for generic environment, error, and origin helpers. Tenant context DTOs and the simple
health response live in `server/kasitek-tenants/src/types.rs`, so the API no longer depends on the
shared `types` crate.

These generic crates remain Labs-owned and are consumed from its private Git repository at the
immutable tag `rust-crates-v0.1.0`. They are not published to a public crate registry. Cargo records
the selected revision in `server/Cargo.lock`. The `config` crate is
still used only for generic helpers; tenant-specific settings remain in the Agency service.
Building Agency requires read access to the private Labs repository, so local contributors and any
trusted build environment need credentials with that access. Credentials must stay outside the
repository.

The service's Analytics publisher calls the Analytics ingestion API using
`ANALYTICS_INGESTION_URL` and tenant publisher credentials. It uses the Labs-owned
`analytics-producer` library for the shared event contract, outbox operations, and authenticated HTTP
client; no Analytics service source was copied into Agency.

## Portal and Growth integration points

- The portal's Next.js route at `/api/tenant/[...path]` proxies to `TENANT_BACKEND_URL`, falling
  back to `NEXT_PUBLIC_TENANT_API_BASE_URL` and then `http://localhost:8090`. It forwards the
  browser cookie and host/proxy headers to the tenant API.
- Portal authentication uses the tenant API session endpoints and cookies. Tenant context is
  selected from the request host; the portal also fetches `/api/public/tenant-context` from its
  current origin. The destination's hosting and route dispatch are not configured by this move.
- Growth retains its admin provisioning consumer. The Growth admin page posts to `/api/agencies`,
  whose route proxies to the tenant API's `/api/admin/workspaces`. Growth's
  `TENANTS_BACKEND_URL` defaults to `http://localhost:8090`, and its proxy forwards
  `TENANT_ADMIN_SECRET` as `x-admin-secret`. Workspace creation, invites, and returned
  portal/domain URLs still cross the repository boundary.
- Growth's admin UI reads `NEXT_PUBLIC_TENANT_PORTAL_URL` for the portal fallback. Tenant domains
  depend on `TENANT_PLATFORM_BASE_DOMAIN` and `TENANT_PORTAL_HOST_TARGET`; cookie behavior depends
  on `TENANT_SESSION_COOKIE_DOMAIN`, `TENANT_SESSION_COOKIE_SECURE`,
  `TENANT_SESSION_COOKIE_SAME_SITE`, and the tenant/client session-cookie names and TTL settings.
  Domain routing, sign-in integration, and production cutover remain deferred to Agency #4 or
  later integration work.

These are follow-up integration paths. This change does not alter their contracts, restore
cross-product journeys, apply migrations, or modify Growth.

## Release impact

- Change and owner: source relocation for the Agency Tenant Platform; Agency owns the moved source.
- Risk: standard; no runtime or production state is changed.
- Affected surfaces: tenant API source, migration source, and tenant portal source.
- Release version or deployment identity: not applicable; no release or deployment is part of this
  change.
- API/schema/migration behavior: no contract or SQL behavior changed; migrations were relocated
  but not applied.
- Rollout and feature flag: not applicable; no rollout occurred.
- Analytics events and success metrics: not applicable; publisher code and event contracts were
  relocated unchanged.
- Logs, metrics, dashboards, and alerts: not applicable; runtime instrumentation was not changed.
- Documentation: this file and the repository README describe the new source layout and commands.
- Validation: touched Rust files pass rustfmt and ownership searches confirm no shared `types`
  dependency remains. Cargo metadata resolves the private Labs crate dependencies and locks their
  source revision in `server/Cargo.lock`; no API build or API tests ran. The portal passed a frozen dependency install,
  typecheck, and production build under Node 22.22.2. `git diff --check` passed. Portal lint remains
  deferred because the destination has no ESLint config and the attempted temporary config exposed
  existing violations; see the source-move PR for the recorded commands and results.
- Rollback: revert the destination source commit; Growth and production remain unchanged.
- Post-release review: not applicable; this is not a release.
