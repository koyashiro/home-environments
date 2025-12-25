# home-environments

## Local Development Setup

```sh
docker compose up -d
export DATABASE_URL='postgresql://home_environments_local@localhost:26257/home_environments_local?sslmode=disable'
sqlx database setup
```

## Connect to CockroachDB SQL Shell

```sh
docker compose exec cockroachdb cockroach sql --insecure
```
