#!/usr/bin/env bash
set -euo pipefail

repository=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
docker build --file "$repository/containers/judge/Dockerfile" --tag whyvec-judge:2026-07-21 "$repository"
docker run --rm whyvec-judge:2026-07-21
