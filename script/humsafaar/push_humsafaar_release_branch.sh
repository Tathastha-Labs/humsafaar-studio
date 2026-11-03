#!/bin/bash

set -euxo pipefail

declare -r RELEASE_BRANCH="humsafaar-release"

if git show-ref --verify --quiet "refs/heads/${RELEASE_BRANCH}"; then
  git branch -D humsafaar-release
fi

git checkout main
git pull

git checkout -b "${RELEASE_BRANCH}"
git push origin "${RELEASE_BRANCH}"

git checkout main

