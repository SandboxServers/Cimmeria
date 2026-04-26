# Stargate Worlds Game Client

This directory is for the Stargate Worlds game client used in local development and testing.

## Base Client

The launcher requires the original game client (base-client) to bootstrap the installation. The base client is the beta release of Stargate Worlds and is available via Internet Archive.

### Obtaining the Base Client

1. Visit the Internet Archive Wayback Machine
2. Search for the Stargate Worlds project archive
3. Locate the file: `Stargate Worlds (0.8348.1.4046) (2009-06-30) (beta).rar`
4. Download the RAR archive

### Extracting the Base Client

**Do not use the installer or Windows File Explorer.** The installer creates unnecessary nested folder structures, and File Explorer / Windows built-in zip tools do not correctly read the CAB file structure—they treat it as a flat directory.

Use 7-Zip (full version only) to extract:

1. Install 7-Zip if not already present
2. Right-click the downloaded RAR file → "7-Zip" → "Extract Here"
3. Locate the first CAB file in the extracted contents (e.g., `data1.cab`)
4. Right-click the first CAB file → "7-Zip" → "Extract Here"
5. 7-Zip will automatically walk through all CAB files and extract the complete game client structure to this directory

The launcher will use these pre-extracted files during initial setup.

### Directory Structure

After extraction, the game client files will be organized here for development and testing purposes.

## Development Usage

The extracted game client cache files in this directory are automatically copied by the bootstrap script for server development:

- When `setup.ps1` runs, it checks this directory for game cache files (pak, dat, res, etc.)
- If present, cache files are copied to `data/cache/` for the server to use
- These cached files support server and patch development activities without requiring additional setup
- Developers working on server features, content, or patch systems have immediate access to game data

This happens automatically during the database initialization step — no manual copy needed.

## Notes

- This is a beta build from 2009 and is provided for development and historical preservation purposes
- The client is read-only for development; modifications are made via patches, not by editing client files
