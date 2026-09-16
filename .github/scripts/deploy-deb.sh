#!/usr/bin/env bash

set -euo pipefail

DEB="$1"
echo "Uploading $DEB"

RESPONSE_FILE="$(mktemp)"
HTTP_CODE="$(curl --silent --output $RESPONSE_FILE --write-out "%{http_code}" -F "$DEB=@$DEB" -H "Token: $DEB_DEPLOY_TOKEN" "$DEB_DEPLOY_URL")"
RESPONSE="$(cat "$RESPONSE_FILE")"

echo "$HTTP_CODE - $RESPONSE"

if [[ ${HTTP_CODE} -lt 200 || ${HTTP_CODE} -gt 299 ]] ; then
    return 22
fi
