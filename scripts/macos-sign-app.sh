#!/usr/bin/env bash
# Sign or verify a macOS .app with a stable code-signing identity.
#
# Why this exists:
# macOS Accessibility (TCC) grants are matched against the app's designated
# code requirement. Ad-hoc signatures usually produce cdhash-only requirements,
# so every binary update can look like a different app even when System Settings
# still shows "Translator" as enabled.

set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  scripts/macos-sign-app.sh [APP_PATH]
  scripts/macos-sign-app.sh --local-dev-identity [APP_PATH]
  scripts/macos-sign-app.sh --verify-only [APP_PATH]

Defaults:
  APP_PATH=/Applications/Translator.app

Required for signing:
  APPLE_SIGNING_IDENTITY or CODESIGN_IDENTITY

Optional certificate import:
  APPLE_CERTIFICATE           base64-encoded .p12 certificate
  APPLE_CERTIFICATE_PASSWORD  password for the .p12 certificate

Optional:
  TRANSLATOR_BUNDLE_ID        defaults to dev.translator.desktop
  TRANSLATOR_CODESIGN_RUNTIME defaults to true
  TRANSLATOR_LOCAL_CERT_DIR   defaults to ~/Library/Application Support/Translator/signing
USAGE
}

fail() {
  echo "error: $*" >&2
  exit 1
}

if [[ "$(uname -s)" != "Darwin" ]]; then
  fail "macOS signing is only available on Darwin"
fi

VERIFY_ONLY=false
LOCAL_DEV_IDENTITY=false
while [[ "${1:-}" == --* ]]; do
  case "$1" in
    --verify-only)
      VERIFY_ONLY=true
      shift
      ;;
    --local-dev-identity)
      LOCAL_DEV_IDENTITY=true
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      fail "unknown option: $1"
      ;;
  esac
done

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

APP_PATH="${1:-${TRANSLATOR_APP_PATH:-/Applications/Translator.app}}"
BUNDLE_ID="${TRANSLATOR_BUNDLE_ID:-dev.translator.desktop}"
IDENTITY="${APPLE_SIGNING_IDENTITY:-${CODESIGN_IDENTITY:-}}"
RUNTIME="${TRANSLATOR_CODESIGN_RUNTIME:-true}"
LOCAL_CERT_NAME="${TRANSLATOR_LOCAL_CERT_NAME:-Translator Local Code Signing}"

[[ -d "$APP_PATH" ]] || fail "app bundle not found: $APP_PATH"
[[ -f "$APP_PATH/Contents/Info.plist" ]] || fail "Info.plist not found in app bundle: $APP_PATH"

verify_signature() {
  local app_path="$1"
  local info requirement

  /usr/bin/codesign --verify --deep --strict --verbose=2 "$app_path" \
    || fail "app signature is invalid or missing; sign it with a stable code-signing identity"
  info="$(/usr/bin/codesign -dv --verbose=4 "$app_path" 2>&1)"
  requirement="$(/usr/bin/codesign -dr - "$app_path" 2>&1)"

  if ! grep -q "Identifier=${BUNDLE_ID}" <<<"$info"; then
    echo "$info" >&2
    fail "code signature identifier does not match ${BUNDLE_ID}"
  fi

  if grep -q 'cdhash H"' <<<"$requirement" && ! grep -q 'identifier "' <<<"$requirement"; then
    echo "$requirement" >&2
    fail "macOS designated requirement is cdhash-only; Accessibility grants will go stale after updates"
  fi

  echo "$info" | sed -n '1,16p'
  echo "$requirement" | sed -n '1,4p'
}

if [[ "$VERIFY_ONLY" == "true" ]]; then
  verify_signature "$APP_PATH"
  exit 0
fi

TMP_DIR="$(mktemp -d)"
KEYCHAIN=""
ORIGINAL_KEYCHAINS_RESTORED=true
ORIGINAL_KEYCHAINS=()
cleanup() {
  if [[ "$ORIGINAL_KEYCHAINS_RESTORED" == "false" ]]; then
    security list-keychains -d user -s "${ORIGINAL_KEYCHAINS[@]}" >/dev/null 2>&1 || true
  fi
  if [[ -n "$KEYCHAIN" && -f "$KEYCHAIN" ]]; then
    security delete-keychain "$KEYCHAIN" >/dev/null 2>&1 || true
  fi
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

decode_base64_certificate() {
  CERT_BASE64="$TMP_DIR/apple-certificate.p12.base64"
  CERT_P12="$TMP_DIR/apple-certificate.p12"
  printf '%s' "$APPLE_CERTIFICATE" >"$CERT_BASE64"
  if ! base64 --decode "$CERT_BASE64" >"$CERT_P12" 2>/dev/null; then
    base64 -D -i "$CERT_BASE64" -o "$CERT_P12"
  fi
}

create_or_reuse_local_certificate() {
  command -v openssl >/dev/null 2>&1 || fail "openssl is required to create a local development signing identity"

  local cert_dir="${TRANSLATOR_LOCAL_CERT_DIR:-$HOME/Library/Application Support/Translator/signing}"
  local password_file="$cert_dir/local-dev.p12.password"
  local cert_p12="$cert_dir/local-dev.p12"
  local cert_pem="$TMP_DIR/local-dev-cert.pem"
  local key_pem="$TMP_DIR/local-dev-key.pem"

  mkdir -p "$cert_dir"
  chmod 700 "$cert_dir"

  if [[ ! -f "$cert_p12" || ! -f "$password_file" ]]; then
    local password
    password="$(uuidgen)"
    printf '%s' "$password" >"$password_file"
    chmod 600 "$password_file"
    openssl req \
      -newkey rsa:2048 \
      -nodes \
      -keyout "$key_pem" \
      -x509 \
      -days 3650 \
      -out "$cert_pem" \
      -subj "/CN=${LOCAL_CERT_NAME}" \
      -addext "keyUsage=digitalSignature" \
      -addext "extendedKeyUsage=codeSigning" >/dev/null 2>&1
    openssl pkcs12 \
      -legacy \
      -export \
      -out "$cert_p12" \
      -inkey "$key_pem" \
      -in "$cert_pem" \
      -passout "pass:${password}" >/dev/null 2>&1
    chmod 600 "$cert_p12"
  fi

  LOCAL_CERT_P12="$cert_p12"
  LOCAL_CERT_PASSWORD="$(cat "$password_file")"
}

prepare_temp_keychain() {
  local cert_p12="$1"
  local cert_password="$2"
  local trust_cert_pem="${3:-}"

  mapfile -t ORIGINAL_KEYCHAINS < <(security list-keychains -d user | sed 's/^ *"//; s/"$//')
  KEYCHAIN="$TMP_DIR/translator-signing.keychain-db"
  KEYCHAIN_PASSWORD="$(uuidgen)"
  security create-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN" >/dev/null
  security unlock-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN" >/dev/null
  security set-keychain-settings -lut 21600 "$KEYCHAIN" >/dev/null
  security list-keychains -d user -s "$KEYCHAIN" "${ORIGINAL_KEYCHAINS[@]}" >/dev/null
  ORIGINAL_KEYCHAINS_RESTORED=false
  security import "$cert_p12" -k "$KEYCHAIN" -P "$cert_password" -T /usr/bin/codesign >/dev/null
  if [[ -n "$trust_cert_pem" ]]; then
    security add-trusted-cert -r trustRoot -p codeSign -k "$KEYCHAIN" "$trust_cert_pem" >/dev/null 2>&1 || true
  fi
  security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "$KEYCHAIN_PASSWORD" "$KEYCHAIN" >/dev/null
  security find-identity -v -p codesigning "$KEYCHAIN" | grep -F "$IDENTITY" >/dev/null \
    || fail "signing identity was not found in the imported certificate"
}

CODESIGN_KEYCHAIN_ARGS=()
if [[ "$LOCAL_DEV_IDENTITY" == "true" ]]; then
  IDENTITY="$LOCAL_CERT_NAME"
  create_or_reuse_local_certificate
  openssl pkcs12 -legacy -in "$LOCAL_CERT_P12" -nokeys -clcerts -passin "pass:${LOCAL_CERT_PASSWORD}" -out "$TMP_DIR/local-dev-cert.pem" >/dev/null 2>&1
  prepare_temp_keychain "$LOCAL_CERT_P12" "$LOCAL_CERT_PASSWORD" "$TMP_DIR/local-dev-cert.pem"
  CODESIGN_KEYCHAIN_ARGS=(--keychain "$KEYCHAIN")
elif [[ -n "${APPLE_CERTIFICATE:-}" ]]; then
  [[ -n "${APPLE_CERTIFICATE_PASSWORD:-}" ]] || fail "APPLE_CERTIFICATE_PASSWORD is required with APPLE_CERTIFICATE"
  decode_base64_certificate
  prepare_temp_keychain "$CERT_P12" "$APPLE_CERTIFICATE_PASSWORD"
  CODESIGN_KEYCHAIN_ARGS=(--keychain "$KEYCHAIN")
fi

[[ -n "$IDENTITY" ]] || fail "set APPLE_SIGNING_IDENTITY or CODESIGN_IDENTITY, or use --local-dev-identity for local testing"
[[ "$IDENTITY" != "-" ]] || fail "ad-hoc signing is not stable enough for macOS Accessibility grants"

CODESIGN_ARGS=(--force --deep --sign "$IDENTITY" --identifier "$BUNDLE_ID")
if [[ "$RUNTIME" != "false" ]]; then
  CODESIGN_ARGS+=(--options runtime)
fi
CODESIGN_ARGS+=("${CODESIGN_KEYCHAIN_ARGS[@]}" "$APP_PATH")

/usr/bin/codesign "${CODESIGN_ARGS[@]}"
verify_signature "$APP_PATH"

echo "Signed $APP_PATH with $IDENTITY"
