# Vault Setup For Tenant DB Secrets

## Acceso rápido

```bash
export VAULT_ADDR="http://127.0.0.1:8200"
```

Si tienes token de admin:

```bash
vault login
```

Si tu Vault usa otro método (OIDC/userpass), ejecuta el login correspondiente.

## 1) Crear policy

```bash
mkdir -p ~/vault
cd ~/vault
```

```bash
nano tenants-policy.hcl
```

Pega esto:

```hcl
path "secret/data/tenants/*" {
  capabilities = ["create", "update", "read"]
}

path "secret/metadata/tenants/*" {
  capabilities = ["delete", "read"]
}
```

## 2) Cargar policy en Vault

```bash
vault policy write tenant-db tenants-policy.hcl
```

## 3) Asignar policy al AppRole `backend`

```bash
vault write auth/approle/role/backend token_policies="tenant-db"
```

## 4) (Opcional) Generar nuevo secret_id

```bash
vault write -f auth/approle/role/backend/secret-id
```

Actualiza `.env` con el `secret_id` nuevo.

## 5) Verificar policy asignada

```bash
vault read auth/approle/role/backend
```

## 6) Probar escritura/lectura

```bash
vault write secret/data/tenants/test/db connection_string="postgres://user:pass@host:5432/db"
vault read secret/data/tenants/test/db
```
