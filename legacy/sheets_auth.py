"""Google Sheets OAuth / service-account authentication for write access."""

from __future__ import annotations

import json
from pathlib import Path

# Repo root — credentials stay next to award_columns.json, not under legacy/.
ROOT = Path(__file__).resolve().parent.parent
SCOPES = ["https://www.googleapis.com/auth/spreadsheets"]
CREDENTIALS_CANDIDATES = (
    ROOT / "credentials.json",
    ROOT / "client_secret.json",
    ROOT / "oauth_credentials.json",
)
SERVICE_ACCOUNT_CANDIDATES = (
    ROOT / "service_account.json",
    ROOT / "service-account.json",
)
TOKEN_PATH = ROOT / "token.json"


class AuthError(RuntimeError):
    pass


def _find_existing(paths: tuple[Path, ...]) -> Path | None:
    for path in paths:
        if path.is_file():
            return path
    return None


def credentials_path() -> Path | None:
    return _find_existing(CREDENTIALS_CANDIDATES)


def service_account_path() -> Path | None:
    return _find_existing(SERVICE_ACCOUNT_CANDIDATES)


def auth_status() -> str:
    if service_account_path():
        return "service_account"
    if TOKEN_PATH.is_file():
        try:
            from google.oauth2.credentials import Credentials

            creds = Credentials.from_authorized_user_file(str(TOKEN_PATH), SCOPES)
            if creds and (creds.valid or (creds.expired and creds.refresh_token)):
                return "oauth_token"
            return "oauth_needs_login"
        except Exception:  # noqa: BLE001
            return "oauth_needs_login"
    if credentials_path():
        return "oauth_needs_login"
    return "missing"


def get_credentials(*, interactive: bool = True):
    """Return Google credentials for Sheets write access.

    Prefers a service account JSON if present; otherwise OAuth desktop flow
    using credentials.json / client_secret.json and a cached token.json.
    """
    try:
        from google.auth.transport.requests import Request
        from google.oauth2.credentials import Credentials
        from google.oauth2 import service_account
        from google_auth_oauthlib.flow import InstalledAppFlow
    except ImportError as exc:
        raise AuthError(
            "Google API packages missing. Run: pip install -r legacy/requirements.txt"
        ) from exc

    sa_path = service_account_path()
    if sa_path:
        return service_account.Credentials.from_service_account_file(
            str(sa_path),
            scopes=SCOPES,
        )

    creds = None
    if TOKEN_PATH.is_file():
        creds = Credentials.from_authorized_user_file(str(TOKEN_PATH), SCOPES)

    if creds and creds.expired and creds.refresh_token:
        try:
            creds.refresh(Request())
        except Exception as exc:  # noqa: BLE001
            raise AuthError(f"Token refresh failed. Run: python3 main.py --login ({exc})") from exc
        TOKEN_PATH.write_text(creds.to_json(), encoding="utf-8")
        try:
            TOKEN_PATH.chmod(0o600)
        except OSError:
            pass
        return creds

    if creds and creds.valid:
        return creds

    oauth_path = credentials_path()
    if not oauth_path:
        raise AuthError(
            "No credentials found. Place OAuth client JSON as credentials.json "
            "(or service_account.json shared on the sheet), then run: "
            "python3 main.py --login"
        )

    if not interactive:
        raise AuthError("Not logged in. Run: python3 main.py --login")

    flow = InstalledAppFlow.from_client_secrets_file(str(oauth_path), SCOPES)
    creds = flow.run_local_server(port=0, prompt="consent")
    TOKEN_PATH.write_text(creds.to_json(), encoding="utf-8")
    try:
        TOKEN_PATH.chmod(0o600)
    except OSError:
        pass
    return creds


def login() -> str:
    """Force interactive OAuth login and cache token.json. Returns account hint."""
    # Remove stale token so --login always re-consents when possible.
    if TOKEN_PATH.exists() and not service_account_path():
        TOKEN_PATH.unlink()
    creds = get_credentials(interactive=True)
    email = ""
    if hasattr(creds, "service_account_email"):
        email = creds.service_account_email or ""
    elif TOKEN_PATH.is_file():
        try:
            data = json.loads(TOKEN_PATH.read_text(encoding="utf-8"))
            email = data.get("account") or data.get("client_id", "")
        except json.JSONDecodeError:
            email = ""
    return email or auth_status()


def build_sheets_service(*, interactive: bool = True):
    try:
        from googleapiclient.discovery import build
    except ImportError as exc:
        raise AuthError(
            "Missing googleapiclient. Activate .venv or run: pip install -r legacy/requirements.txt"
        ) from exc

    creds = get_credentials(interactive=interactive)
    return build("sheets", "v4", credentials=creds, cache_discovery=False)
