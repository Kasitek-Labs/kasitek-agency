# Kasitek Agency

Agency owns the tenant API, its database schema and migrations, and the tenant portal.

## Source layout

- `server/kasitek-tenants/` — Tenant Platform API and SQL migrations.
- `client/apps/tenant-portal/` — tenant and client portal.
- `client/packages/api-client-react/` — the portal's required tenant API client.

## Local commands

Install and run the portal:

```sh
corepack pnpm --dir client install --frozen-lockfile
corepack pnpm --dir client dev
```

The API command is:

```sh
cargo run --manifest-path server/Cargo.toml -p kasitek-tenants
```

The API source still depends on five shared Rust crates whose ownership is not settled. Cargo
checks cannot complete until those dependencies are made available to Agency; see
[source boundaries](docs/source-boundaries.md).

## Ownership and integration

The source move does not change runtime behavior or deploy either service. Growth's admin-side
provisioning consumer remains in Growth. Portal routing, authentication cookies, tenant domains,
Analytics publication, and the shared Rust crate boundary are recorded in
[source boundaries](docs/source-boundaries.md).
