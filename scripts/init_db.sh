#!/usr/bin/env bash
set -euxo pipefail

DB_USER="${POSTGRES_USER:=postgres}"
DB_PASSWORD="${POSTGRES_PASSWORD:=password}"
DB_NAME="${POSTGRES_DB:=adventus}"
DB_HOST="${POSTGRES_HOST:=localhost}"
DB_PORT="${POSTGRES_PORT:=5432}"

running_postgres_containers=$(docker ps --quiet --filter ancestor=postgres)

if [ -z "${running_postgres_containers}" ]; then
	echo "Starting PostgreSQL container"
	docker run --env POSTGRES_PASSWORD="${DB_PASSWORD}" --env POSTGRES_DB="${DB_NAME}" --publish "${DB_PORT}":5432 --detach postgres
else
	echo "PostgreSQL is already running"
fi

until pg_isready --host="${DB_HOST}" --port="${DB_PORT}"; do
	echo "Waiting for PostgreSQL database to be ready..."
	sleep 1
done

# sqlx requires DATABASE_URL
export DATABASE_URL=postgres://${DB_USER}:${DB_PASSWORD}@${DB_HOST}:${DB_PORT}/${DB_NAME}

sqlx migrate run

echo "PostgreSQL has been migrated, ready to go!"
