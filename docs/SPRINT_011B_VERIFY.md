# Sprint 011B Live Verification

1. In the EVE Developer application, confirm the callback remains exactly
   `http://localhost:38473/callback`.
2. Start RenderNorth Industrial and open Settings → Characters.
3. Confirm Maxdelta and Saber Side show `reauthorize`. They were connected
   before `esi-assets.read_assets.v1` was requested.
4. Select **Add / Reauthorize Character**, authenticate Maxdelta, review the
   read-assets permission, and accept it. Repeat for Saber Side.
5. Confirm both characters show `authorized` without a reauthorization badge.
6. Select **Sync** on Maxdelta. Confirm status becomes `success`, Last Sync is
   populated, and asset/page counts are greater than zero if that character
   owns assets.
7. Disable Saber Side and select **Sync All Enabled Characters**. Confirm only
   Maxdelta's sync metadata changes.
8. Re-enable Saber Side and select **Sync All Enabled Characters**. Confirm
   both characters report `success` with asset and page counts.
9. Open Inventory → Synchronized ESI Assets. Confirm rows show owner, type,
   quantity, item ID, raw location ID, location type, location flag, singleton,
   source, and last-synced time.
10. Open a real production plan containing a type owned by either enabled
    character. Confirm Globally Owned includes manual inventory plus enabled
    synchronized-character quantities. Disable that character and recalculate;
    confirm its contribution is removed while its saved snapshot remains.
11. To verify failure preservation without deleting credentials, interrupt
    network access after a successful sync and select Sync again. Confirm the
    character reports an error and the previously synchronized Inventory rows
    remain present.

Tokens must never appear in the UI, application logs, SQLite, or error text.
