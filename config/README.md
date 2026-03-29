# config/ — Service Configuration

XML configuration files for the C++ server services. The Rust server reads these same files at startup.

3 files.

## Files

| File | Service | Key Settings |
|---|---|---|
| `AuthenticationService.config` | AuthenticationServer | Port (13001), encryption keys |
| `BaseService.config` | BaseApp | DB connection string, shard settings, port (32832) |
| `CellService.config` | CellApp | World cell parameters, AoI distances, grid settings |

## Usage

Files contain **default/template values** suitable for local development. For production or multi-machine setups, create `*.local` override files (gitignored) alongside the base configs.

**Database connection** (`BaseService.config`): The `db_connection_string` field uses test credentials by default. Override with a `.local` file for real environments.

**Python console**: Port 8989 is available for live Python debugging. Requires a password to be set — see `AuthenticationService.config`.

**Developer mode**: A flag in the config enables relaxed auth validation and elevated logging. Enable for local development.

## Environment Variable Overrides

The Rust server (`crates/common/`) supports environment variable overrides for all config values. Prefix with `CIMMERIA_` — e.g., `CIMMERIA_DB_CONNECTION_STRING`.
