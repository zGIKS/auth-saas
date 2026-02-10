# AdminIdentity Bounded Context

Language-agnostic, framework-agnostic, architecture-focused documentation.

## 0. Documentation Principles

- One Bounded Context has one source of truth: this document.
- Business language is mandatory across code, docs, and conversations.
- Explain why first, then how.
- Boundaries are explicit, including what is not owned here.
- Domain, application, and interface concerns are separated.
- Capabilities and prohibitions are both documented.

---

## 1. Bounded Context Overview

Bounded Context Name: `AdminIdentity`

Purpose:
Provide secure authentication for the platform administrator and controlled credential recovery for operational continuity.

Primary Business Capability:
Single-admin access control for platform-level operations.

Out of Scope:
- End-user registration and sign-in.
- Tenant user lifecycle.
- Role management for multiple admin profiles.
- Tenant-scoped identity concerns.

Boundary Decision:
`AdminIdentity` exists to prevent reusing tenant or end-user authentication flows for privileged platform administration.

---

## 2. Ubiquitous Language

| Term | Definition | Notes |
|---|---|---|
| Admin Account | The unique platform administrator identity. | Single-admin model. |
| Bootstrap | First-time creation of the initial admin account. | One-time operation. |
| Recovery | Controlled replacement of admin credentials when access is lost. | Break-glass operation. |
| Admin Login | Authentication flow that issues an admin token. | Separate from tenant user login. |
| Admin Guard | Policy that requires valid admin token for privileged operations. | Applied to tenant creation. |
| Identity Lockout | Temporary block after repeated failed admin login attempts. | Scope: identity + IP. |

Language Rule:
Inside this context, use `Admin Login`, not `Sign-In`, to avoid collision with end-user authentication terminology.

---

## 3. Domain Model Documentation

### Domain Concept: Admin Account

Description:
Represents the privileged operator identity used for platform administration.

Invariant Rules:
- At most one admin account must exist.
- Stored password is always a password hash, never plain text.
- Stored username is always a deterministic hash representation.

Business Examples:
- During initial deployment, bootstrap creates the first admin account.
- If credentials are lost, recovery replaces credentials for the existing admin account.

### Domain Concept: Admin Username

Description:
Credential identifier provided by operator and transformed to a hashed value for storage and lookup.

Invariant Rules:
- Input username must satisfy admin username policy before hashing.
- Persisted form is hash-only.

### Domain Concept: Admin Password

Description:
Secret credential for admin authentication.

Invariant Rules:
- Minimum security policy is enforced before processing.
- Persisted form is password hash-only.

---

## 4. Commands Documentation

### Command Name: `CreateInitialAdmin`

Intent:
Create the first and only admin account.

Required Data:
- Username
- Password

Business Rules:
- Command is valid only when no admin account exists.
- Username and password policies must pass.
- Persisted values must be stored as hashes.

Possible Rejections:
- Initial admin already exists.
- Credential policy violation.
- Persistence failure.

### Command Name: `AdminLogin`

Intent:
Authenticate admin and issue access token.

Required Data:
- Username
- Password
- Request source information (IP) for lockout policy.

Business Rules:
- Login is denied when identity is locked.
- Username is transformed to deterministic hash for lookup.
- Password is verified against password hash.

Possible Rejections:
- Invalid credentials.
- Lockout active.
- Internal authentication failure.

---

## 5. Queries Documentation

### Query Name: `FindAdminByUsername`

Information Requested:
Retrieve admin account by hashed username identifier.

Filters:
- Hashed username.

Constraints:
- Deterministic lookup only.

Returned Data:
- Optional admin account identity record.

### Query Name: `CountAdminAccounts`

Information Requested:
Count existing admin accounts.

Constraints:
- Used to enforce single-admin invariant.

Returned Data:
- Integer count.

---

## 6. Domain Events Documentation

### Event Name: `InitialAdminCreated`

Business Meaning:
The platform now has an initial administrator configured.

Triggered When:
`CreateInitialAdmin` succeeds.

Consumers:
- Operational audit and startup verification processes.

Data Carried:
- Admin account identifier
- Occurrence timestamp

---

## 7. Domain Services Documentation

### Service Name: `AdminIdentityCommandService`

Business Capability:
Execute admin state-changing actions (`CreateInitialAdmin`, `AdminLogin`).

Inputs:
- Admin commands.

Outputs:
- Domain events or admin access token.

Business Rules:
- Enforces single-admin constraint.
- Enforces credential and lockout policies.

### Service Name: `AdminIdentityQueryService`

Business Capability:
Provide read-side access for admin account discovery and invariant checks.

Inputs:
- Admin queries.

Outputs:
- Optional admin account and aggregate counters.

Business Rules:
- Read-only behavior.

---

## 8. Persistence & Repositories Documentation

Aggregate:
Admin Account

Persistence Responsibility:
Store and retrieve the single admin account with hashed credentials.

Consistency Rules:
- Only one admin account is allowed by business policy.
- Writes maintain auditable timestamps.
- Username and password are stored hashed.

Loading Strategy:
- Deterministic identity lookup by hashed username.
- Count query for invariant checks.

---

## 9. Application Layer Documentation

Application Service:
Admin command and query orchestrators.

Responsibility:
Coordinate domain rules, hashing/verification, lockout checks, and token issuance.

Flow Description:
- Validate intent and policies.
- Resolve aggregate data from repository.
- Execute command outcomes.
- Return token/event or controlled failure.

Transactional Boundaries:
- Each command executes as a single logical operation from validation to persistence/token response.

Error Handling Strategy:
- Domain-relevant failures are returned as explicit business errors.
- Internal failures are generalized for external consumers.

---

## 10. Interfaces / API Documentation

Endpoint / Interface:
`POST /api/v1/admin/login`

Purpose:
Authenticate administrator and issue admin access token.

Input Contract:
- Username
- Password

Output Contract:
- Access token

Error Scenarios:
- Invalid request payload
- Invalid credentials
- Temporary lockout
- Internal failure

Deliberate Hiding:
- No admin identifiers or internal state details are exposed in API responses.
- Credential storage format is never exposed.

---

## 11. Anti-Corruption Layer (ACL) Documentation

### 11.1 Context Relationship

Consumer Context:
`Tenancy`

Provider Context:
`AdminIdentity`

Relationship Type:
Authorization dependency (privileged operation guard).

### 11.2 Translation Rules

External Concept -> Internal Concept:
- `Authorization Bearer token` -> `Admin identity claim (subject)`

Transformation Rules:
- Validate token authenticity and validity.
- Resolve subject into admin account existence.
- Expose only allow/deny semantics to consumer context.

### 11.3 Failure Handling

External Failure:
- Missing token, invalid token, unknown admin subject.

Internal Reaction:
- Reject privileged operation.

Fallback Strategy:
- No fallback to tenant or public auth paths.

---

## 12. Context Boundaries & Integration Map

Upstream / Downstream:
- `AdminIdentity` is authoritative for admin authentication.
- `Tenancy` consumes admin authorization for tenant creation.

Published Language:
- Admin token as privileged access artifact.

ACL Boundaries:
- `Tenancy` does not read or manipulate admin credentials directly.
- `Tenancy` only depends on validated admin identity outcome.

Integration Decision:
Tenant creation is explicitly protected and cannot proceed without a valid admin token.

---

## Operational Commands

Run API:
```bash
cargo run
```

Create initial admin (first setup only):
```bash
cargo run --bin admin_identity_bootstrap_cli
```

Recover admin access (replace credentials):
```bash
cargo run --bin admin_identity_recover_cli
```

---

## Code References

- `src/iam/admin_identity/interfaces/rest/controllers/admin_authentication_controller.rs`
- `src/iam/admin_identity/application/command_services/admin_identity_command_service_impl.rs`
- `src/iam/admin_identity/application/query_services/admin_identity_query_service_impl.rs`
- `src/iam/admin_identity/domain/model/aggregates/admin_account.rs`
- `src/iam/admin_identity/domain/repositories/admin_account_repository.rs`
- `src/tenancy/interfaces/rest/admin_guard_middleware.rs`
- `src/bin/admin_identity_bootstrap_cli.rs`
- `src/bin/admin_identity_recover_cli.rs`
