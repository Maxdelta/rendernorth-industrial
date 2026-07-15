# RenderNorth Industrial Private Beta Onboarding

RenderNorth Industrial is a local-first industrial intelligence application for EVE Online. It never automates gameplay, never controls the EVE client, and uses only CCP-authorized read-only information.

## Before first launch

1. Create a native EVE Developer application at the official EVE Developers site.
2. Set its callback URL to `http://localhost:17117/callback`.
3. Keep the Client ID available. RenderNorth does not use a client secret.
4. Download and extract the official CCP JSONL static-data export. Keep the extracted folder containing `categories.jsonl`, `groups.jsonl`, `types.jsonl`, and `blueprints.jsonl`.

## First-run sequence

1. Launch RenderNorth Industrial and read the privacy disclosure.
2. Enter the CCP Developer Client ID.
3. Browse to the extracted CCP SDE directory, validate it, and import it.
4. Add or reauthorize each character through the official CCP browser flow.
5. Refresh assets and locations.
6. Refresh blueprints.
7. Refresh the selected market snapshot.
8. Finish setup and open Quartermaster.
9. Create a doctrine, import an EFT fit, choose fleet quantities, and select **Analyze Doctrine**.

## Local data and credentials

- Application data is stored locally in the RenderNorth SQLite database.
- EVE access and refresh tokens never enter the frontend.
- Refresh tokens remain in Windows Credential Manager.
- RenderNorth does not receive or store EVE usernames or passwords.
- Exported diagnostics exclude tokens, Client IDs, credentials, and personal secrets.

## Support

- GitHub: https://github.com/Maxdelta/rendernorth-industrial
- Issues: https://github.com/Maxdelta/rendernorth-industrial/issues
- Website: https://rendernorth.com
