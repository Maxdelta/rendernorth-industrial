# RenderNorth Industrial Open Beta 0.1 Onboarding

## Normal setup

1. Install and launch RenderNorth Industrial.
2. Select **Connect Character**. The built-in public RenderNorth Industrial CCP application opens CCP's official login; no developer account or Client ID is required.
3. Download the supported JSONL archive from the [official CCP static-data page](https://developers.eveonline.com/static-data).
4. Extract the ZIP to a permanent folder, then select the extracted folder. It must contain `categories.jsonl`, `groups.jsonl`, `types.jsonl`, and `blueprints.jsonl`.
5. Sync assets, blueprints, and locations; refresh the market; finish setup.

RenderNorth requests read-only ESI scopes, uses Authorization Code + PKCE, stores refresh tokens in Windows Credential Manager, never receives EVE credentials, and never asks for a Client Secret.

## Application updates

RenderNorth Industrial checks the official public GitHub Releases API shortly after startup, no more often than the selected Daily or Weekly interval. The check is non-blocking and sends no EVE, inventory, Client ID, token, diagnostics, or machine-identifying data. Settings → Updates can disable automatic checks, include or exclude pre-releases, run Check Now, and restore skipped-version notifications.

Download actions open the official GitHub release or installer URL in the default browser. RenderNorth Industrial does not download or install updates internally. Offline or failed checks preserve cached release information and never prevent normal use.

## Advanced authentication

Only users intentionally managing their own CCP application need this path. Create a native application in CCP's developer portal with callback `http://localhost:17117/callback`, copy its public Client ID, expand **Advanced Authentication**, and select **Use Custom Application**. Do not enter, store, or share a Client Secret. **Restore Official RenderNorth Client ID** returns to the built-in application.

## Static-data troubleshooting

- **ZIP selected:** extract it and select the extracted folder.
- **Wrong archive/format:** download the JSONL export, not YAML or a third-party package.
- **Wrong folder level:** select the folder directly containing all four required `.jsonl` files.
- **Incomplete extraction:** extract again and confirm every required file reports **Found**.
- **Remembered path missing:** move the data back or browse to its new permanent location.

## Copy-ready Open Beta announcement

RenderNorth Industrial Open Beta 0.1 is available for Windows. Connect an EVE character through secure read-only CCP SSO, import CCP's official JSONL static data, synchronize inventory and blueprints, value markets, plan production and procurement, and analyze EFT doctrines locally. Report beta issues through GitHub or contact Maxdelta on Discord (`maxdelta0089`).

## Copy-ready Getting Started

Install RenderNorth Industrial, connect a character with the built-in official CCP application, download and extract CCP's JSONL static data, select the extracted folder, then follow the wizard through assets, blueprints, locations, and market refresh. No Git, developer tools, or CCP developer account is required.

## Copy-ready release notes

Open Beta 0.1 delivers first-run onboarding, official read-only character authentication, static-data validation, synchronized assets and blueprints, resolved locations, inventory and production economics, procurement exports, Quartermaster doctrine readiness, diagnostics, and Windows packaging. This is beta software and is not production-certified.

## Help and support

- Bugs: [GitHub Issues](https://github.com/Maxdelta/rendernorth-industrial/issues)
- Discord: `maxdelta0089` or [RenderNorth Discord](https://discord.gg/XycCz6ppx)
- Optional support: [Buy Maxdelta a Coffee](https://buymeacoffee.com/maxdelta)
- In EVE: Character Maxdelta — ISK or PLEX

Support is optional and never required. Discord contact is community support, not official CCP support.
